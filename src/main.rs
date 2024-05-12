// export OIDN_DIR="/media/jakubvondra/Data/apps/oidn/oidn-2.2.2.x86_64.linux"
// export LD_LIBRARY_PATH="/media/jakubvondra/Data/apps/oidn/oidn-2.2.2.x86_64.linux/lib"
// cargo run --release -- -b tests/beauty.####.exr -a tests/denoising_albedo.####.exr -n tests/denoising_normal.####.exr

use clap::{Arg, Command};
use glob::glob;
use hashbrown::HashMap;
use oidn;

mod image;

fn main() {
    let matches = Command::new("My Test Program")
        .version("0.1.0")
        .author("Hackerman Jones <hckrmnjones@hack.gov>")
        .about("Teaches argument parsing")
        .arg(Arg::new("beauty")
                 .short('b')
                 .long("beauty")
                 .help("a beauty .exr file or sequence using the foo.####.exr pattern"))
        .arg(Arg::new("albedo")
                 .short('a')
                 .long("albedo")
                 .help("a albedo .exr file or sequence using the foo.####.exr pattern"))
        .arg(Arg::new("normal")
                 .short('n')
                 .long("normal")
                 .help("a normal .exr file or sequence using the foo.####.exr pattern"))
        .arg(Arg::new("layerbeauty")
                 .short('e')
                 .long("layerbeauty")
                 .help("OPTIONAL - name of layer to denoise inside the beauty exr, defaults to main_layer"))
        .arg(Arg::new("layeralbedo")
                 .short('l')
                 .long("layeralbedo")
                 .help("OPTIONAL - name of layer to denoise inside the albedo exr, defaults to main_layer"))
        .arg(Arg::new("layernormal")
                 .short('o')
                 .long("layernormal")
                 .help("OPTIONAL - name of layer to denoise inside the normal exr, defaults to main_layer"))
        .get_matches();

    let beauty_path: &String = matches.get_one::<String>("beauty").expect("supply beauty exr file(s) using the -b flag");
    let albedo_path: Option<&String> = matches.get_one::<String>("albedo");
    let normal_path: Option<&String> = matches.get_one::<String>("normal");
    
    let default_layer_name = "main_layer".to_string();
    
    let beauty_layer: &String= matches.get_one::<String>("layerbeauty").unwrap_or(&default_layer_name );
    let albedo_layer: &String= matches.get_one::<String>("layeralbedo").unwrap_or(&default_layer_name );
    let normal_layer: &String= matches.get_one::<String>("layernormal").unwrap_or(&default_layer_name );
    
   let (beauty_seq, albedo_seq, normal_seq) ={
    
    if beauty_path.contains("#"){
        let beauty_seq = gather_file_sequence(beauty_path.clone());
        let albedo_seq = match albedo_path{
            Some(path) => { Some(gather_file_sequence(albedo_path.unwrap().clone()) )},
            None => {None}
        };
        let normal_seq = match normal_path{
            Some(path) => { Some(gather_file_sequence(normal_path.unwrap().clone()) )},
            None => {None}
        };
        
        (beauty_seq,albedo_seq,normal_seq)
    }
    else{
        let mut beauty_seq: Vec<String> = Vec::new();
        beauty_seq.push(beauty_path.clone());
        
        let albedo_seq = match albedo_path{
            Some(path) => { 
                let mut h: Vec<String> = Vec::new();
                h.push(path.clone());
                Some(h)
                },
            None => {None}
        };
        let normal_seq = match normal_path{
            Some(path) => { 
                let mut h: Vec<String> = Vec::new();
                h.push(path.clone());
                Some(h)
                },
            None => {None}
            };
            
            (beauty_seq,albedo_seq,normal_seq)
        }   
    };
    
    // sequence sanity check
    if  *&albedo_seq.is_some() || *&normal_seq.is_some(){
        if &beauty_seq.clone().len() != &albedo_seq.clone().unwrap().len() || &beauty_seq.clone().len() != &normal_seq.clone().unwrap().len(){
            panic!("sequences dont have the same frame count!")
        }
    }
    
    //println!("{:?}", &beauty_seq);
    //println!("{:?}", &albedo_seq);
    //println!("{:?}", &normal_seq);

    // Denoise
    let device = oidn::Device::new();
    for (i,beauty_file_path) in beauty_seq.iter().enumerate(){
        let mut beauty_img = image::FloatImage::from_exr(beauty_file_path.clone());
        
        let mut denoiser = oidn::RayTracing::new(&device);
        denoiser
            .srgb(false)
            .hdr(true)
            .image_dimensions(beauty_img.width, beauty_img.height);   
        
        let albedo_data = match albedo_seq.clone(){
            Some(seq)=>{
                let albedo_img = image::FloatImage::from_exr(seq[i].clone());
                let (alebdo_type, albedo_data) = albedo_img.buffers.get(albedo_layer).expect(&format!("layer not found in albedo: {}", albedo_layer)[..]);
                Some(albedo_data.clone())
            },
            None => {None}
        };
        
        match normal_seq.clone(){
            Some(seq)=>{
                let normal_img = image::FloatImage::from_exr(seq[i].clone());
                let (normal_type, normal_data) = normal_img.buffers.get(normal_layer).expect(&format!("layer not found in normal: {}", normal_layer)[..]);
                denoiser.albedo_normal(&albedo_data.expect("Albedo data missing"), &normal_data);
            },
            None => {}
        };
        
        let (beauty_type,mut beauty_data) = beauty_img.buffers.get(beauty_layer).expect(&format!("layer not found in beauty: {}", beauty_layer)[..]).clone();
        let (mut beauty_data_rgb, mut beauty_data_a ) = match beauty_type {
            image::BufferType::RGBA => image::strip_alpha(beauty_data),
            _ => (beauty_data,Vec::new()),
        };
        
        denoiser
            .filter_in_place(&mut beauty_data_rgb)
            .expect("Invalid input image dimensions?");
    
        if let Err(e) = device.get_error() {
            println!("Error denosing image: {}", e.1);
        }

        let beauty_data_denoised = match beauty_type {
            image::BufferType::RGBA => image::add_alpha(beauty_data_rgb, beauty_data_a),
            _ => beauty_data_rgb,
            };
        
        beauty_img.buffers.insert(beauty_layer.clone(), (beauty_type, beauty_data_denoised) );
        
        let out_file_path = beauty_file_path.replace(".exr", "_denoised.exr");
        beauty_img.save_to_file(out_file_path, beauty_layer);
    }

}

fn gather_file_sequence(path: String)->Vec<String>{
    let hashes_location = (path.find("#").unwrap(), path.rfind("#").unwrap());
    let hashes: String = (0..(hashes_location.1-hashes_location.0+1)).map(|i|{"#".to_string()}).collect::<Vec<String>>().join(""); // e.g ####
    let glob_pat: String = (0..(hashes_location.1-hashes_location.0+1)).map(|i|{"?".to_string()}).collect::<Vec<String>>().join(""); // e.g. ????
    //println!("{:?}",hashes);
    
    let mut out: Vec<String> = Vec::new();
    for entry in glob((path.replace(hashes.as_str(), glob_pat.as_str())).as_str()).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                //println!("{:?}", path.display());
                let path_string = path.into_os_string().into_string().unwrap();
                out.push(path_string);
            },
            Err(e) => println!("{:?}", e),
        };
    }
    out
}