use exr::prelude::RgbaImage as rgb_exr;
use hashbrown::HashMap;

#[derive(Clone)]
pub enum BufferType {
    RGB,
    RGBA,
    Other,
}

#[derive(Clone)]
pub struct FloatImage {
    pub width: usize,
    pub height: usize,
    pub buffers: HashMap<String, (BufferType, Vec<f32>)>,
}
impl FloatImage {
    pub fn new(
        width: usize,
        height: usize,
        buffers: HashMap<String, (BufferType, Vec<f32>)>,
    ) -> FloatImage {
        FloatImage {
            width: width,
            height: height,
            buffers: buffers,
        }
    }

    pub fn from_exr(exr_path: String) -> FloatImage {
        use exr::prelude::*;
        let path = exr_path.as_str();
        let mut out_buffers: HashMap<String, (BufferType, Vec<f32>)> = HashMap::new();

        let image = read()
            .no_deep_data()
            .largest_resolution_level()
            .all_channels()
            .all_layers()
            .all_attributes()
            .from_file(path)
            .unwrap();

        let size = image.layer_data[0].size;
        //println!("size: {:?}", size);
        let mut n_pixels = 0 as usize;

        for (layer_index, layer) in image.layer_data.iter().enumerate() {
            let layer_name = layer
                .attributes
                .layer_name
                .as_ref()
                .map_or(String::from("main_layer"), Text::to_string);
            //println!("layer: {:?}",layer_name);

            let mut channels: Vec<Vec<f32>> = Vec::new(); // to temporally store each channel as its own vector

            for channel in &layer.channel_data.list {
                //println!("channel: {:?}",channel.name );
                let channel_values: Vec<f32> = channel.sample_data.values_as_f32().collect();
                n_pixels = channel_values.len();
                channels.push(channel_values);
            }

            // merge the channels into one vec in R,G,B,A,R,G,B,A,R,G,B,A format
            let mut out_buffer: Vec<f32> = Vec::new();
            for pixel_index in 0..n_pixels {
                for ch_data in channels.iter().rev() {
                    out_buffer.push(ch_data[pixel_index])
                }
            }

            let buffer_type = match (out_buffer.len() / (size.x() * size.y())) {
                3 => BufferType::RGB,
                4 => BufferType::RGBA,
                _ => BufferType::Other,
            };

            out_buffers.insert(layer_name.clone(), (buffer_type, out_buffer));
        }

        FloatImage {
            width: size.0,
            height: size.1,
            buffers: out_buffers,
        }
    }

    pub fn save_to_file(&self, out_file: String, out_layer: &String) {
        let (layer_type, layer_data) = self
            .buffers
            .get(out_layer)
            .expect("couldn't find layer in multi_image");

        let get_pixel_rgb = |x: usize, y: usize| {
            let pixel_i = x as usize + (y as usize * *&self.width as usize);
            (
                layer_data[pixel_i * 3],
                layer_data[pixel_i * 3 + 1],
                layer_data[pixel_i * 3 + 2],
            )
        };

        let get_pixel_rgba = |x: usize, y: usize| {
            let pixel_i = x as usize + (y as usize * *&self.width as usize);
            (
                layer_data[pixel_i * 4],
                layer_data[pixel_i * 4 + 1],
                layer_data[pixel_i * 4 + 2],
                layer_data[pixel_i * 4 + 3],
            )
        };

        // write a file without alpha and 32-bit float precision per channel
        match layer_type {
            BufferType::RGB => {
                exr::prelude::write_rgb_file(
                    &out_file,
                    *&self.width as usize,
                    *&self.height as usize, // write an image with this resolution
                    |x, y| {
                        (
                            // generate an f32 rgb color for each of the  pixels
                            get_pixel_rgb(x, y)
                        )
                    },
                )
                .unwrap();
            }
            BufferType::RGBA => {
                exr::prelude::write_rgba_file(
                    &out_file,
                    *&self.width as usize,
                    *&self.height as usize, // write an image with this resolution
                    |x, y| {
                        (
                            // generate an f32 rgb color for each of the  pixels
                            get_pixel_rgba(x, y)
                        )
                    },
                )
                .unwrap();
            }
            _ => {}
        }

        println!("created file {:?}", out_file);
    }
}

pub fn strip_alpha(rgba_data: Vec<f32>) -> (Vec<f32>, Vec<f32>) {
    let mut rgb_data = Vec::new();
    let mut a_data = Vec::new();

    for rgba in rgba_data.chunks(4) {
        rgb_data.push(rgba[0]);
        rgb_data.push(rgba[1]);
        rgb_data.push(rgba[2]);
        a_data.push(rgba[2]);
    }

    (rgb_data, a_data)
}

pub fn add_alpha(rgb_data: Vec<f32>, a_data: Vec<f32>) -> Vec<f32> {
    let mut rgba_data = Vec::new();
    for rgb_a in rgb_data.chunks(3).zip(a_data) {
        rgba_data.push(rgb_a.0[0]);
        rgba_data.push(rgb_a.0[1]);
        rgba_data.push(rgb_a.0[2]);
        rgba_data.push(rgb_a.1);
    }
    rgba_data
}
