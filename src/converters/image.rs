
use image::ImageFormat;

pub trait ImageConverter {
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

pub struct BmpConverter;

impl ImageConverter for BmpConverter {
    fn convert(&self, _data: &mut Vec<u8>, _width: u32, _height: u32) {
        /*
        Since we always get Vec<u8> consistant of BMP data, we will not convert from BMP to BMP.
        This is just the placeholder to make the strategy pattern work.
         */
    }
}

pub struct ImageProcessor;

impl ImageProcessor {
    pub fn process_image(format: ImageFormat, data: &mut Vec<u8>, width: u32, height: u32) {
        match format {
            image::ImageFormat::Bmp => BmpConverter.convert(data, width, height),
            image::ImageFormat::WebP => WebPConverter.convert(data, width, height),
            _ => {}
        }
    }
}
