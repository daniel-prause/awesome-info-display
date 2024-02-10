use std::sync::{Arc, Mutex};

pub trait ImageConverter: Send {
    fn convert(&self, data: &mut Vec<u8>, width: u32, height: u32);
}

pub struct WebPConverter;

impl ImageConverter for WebPConverter {
    fn convert(&self, data: &mut Vec<u8>, width: u32, height: u32) {
        *data = webp::Encoder::new(&*data, webp::PixelLayout::Rgb, width, height)
            .encode(100.0)
            .to_vec();
    }
}

pub struct NoOpConverter;

impl ImageConverter for NoOpConverter {
    fn convert(&self, _data: &mut Vec<u8>, _width: u32, _height: u32) {
        /* this can and will be used, if no operation is necessary */
    }
}

pub struct ImageProcessor {
    converter: Arc<Mutex<dyn ImageConverter>>,
}

impl ImageProcessor {
    pub fn new(converter: Arc<Mutex<dyn ImageConverter>>) -> Self {
        ImageProcessor { converter }
    }

    pub fn process_image(&self, data: &mut Vec<u8>, width: u32, height: u32) {
        self.converter.lock().unwrap().convert(data, width, height);
    }
}
