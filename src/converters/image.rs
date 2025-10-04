use crate::convert_to_gray_scale;

pub trait ImageConverter: Send + Sync {
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

pub struct GrayscaleConverter;

// since the grayscale converter is only used for the teensy, we will adjust the brightness here for now.
impl ImageConverter for GrayscaleConverter {
    fn convert(&self, data: &mut Vec<u8>, _width: u32, _height: u32) {
        *data = convert_to_gray_scale(&data);
    }
}

/*
pub struct NoOpConverter;

impl ImageConverter for NoOpConverter {
    fn convert(&self, _data: &mut Vec<u8>, _width: u32, _height: u32) {
        /* this can and will be used, if no operation is necessary */
    }
}
*/
pub struct ImageProcessor {
    converter: Box<dyn ImageConverter>,
    width: u32,
    height: u32,
}

impl ImageProcessor {
    pub fn new(converter: Box<dyn ImageConverter>, width: u32, height: u32) -> Self {
        ImageProcessor {
            converter,
            width,
            height,
        }
    }

    pub fn process_image(&self, data: &mut Vec<u8>) {
        self.converter.convert(data, self.width, self.height);
    }

    pub fn screen_width(&self) -> u32 {
        return self.width;
    }

    pub fn screen_height(&self) -> u32 {
        return self.height;
    }
}
