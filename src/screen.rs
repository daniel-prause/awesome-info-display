use image::DynamicImage;
use image::ImageBuffer;
use image::Rgb;
use image::RgbImage;
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};
use sysinfo::RefreshKind;
use sysinfo::{System, SystemExt};
#[derive(Debug)]
struct Error1;
#[derive(Debug)]
struct Error2;

#[derive(Debug, Clone, Default)]
pub struct Screen {
    description: String,
    current_image: ImageBuffer<Rgb<u8>, Vec<u8>>,
    bytes: Vec<u8>,
    font: Option<Font<'static>>,
}

impl Screen {
    pub fn new(description: String) -> Self {
        let image = RgbImage::new(256, 64);
        let bytes = Vec::new();
        let font = Vec::from(include_bytes!("DejaVuSans.ttf") as &[u8]);
        let font = Font::try_from_vec(font);

        Screen {
            description,
            current_image: image,
            bytes: bytes,
            font: font,
        }
    }
    pub fn description(&self) -> &String {
        &self.description
    }

    pub fn current_image(&self) -> Vec<u8> {
        self.bytes.clone()
    }
    pub fn update(&mut self) {
        //let mut image = RgbImage::new(256, 64);
        let mut image = RgbImage::new(256, 64);
        let height = 16.0;
        let scale = Scale {
            x: height,
            y: height,
        };
        //let text = Template::new("CPU: {{cpu}}% / Hallo Chris =)");
        //let mut args = HashMap::new();
        //args.insert("cpu", load_avg.one.to_string());
        let refresh_kind = RefreshKind::new();
        let refresh_kind = refresh_kind.with_cpu();
        let refresh_kind = refresh_kind.with_memory();
        let sys = System::new_with_specifics(refresh_kind);
        let load_avg = sys.get_load_average();
        let total_memory = sys.get_total_memory();
        let total_memory = total_memory as f64;
        let free_memory = sys.get_free_memory();
        let free_memory = free_memory as f64;
        let text = format!(
            "CPU: {cpu}% / RAM: {memory}%",
            cpu = (load_avg.one * 100.0).floor(),
            memory = (100.0 - (free_memory / total_memory) * 100.0).floor()
        )
        .to_string();
        let font = self.font.as_ref().unwrap();
        draw_text_mut(
            &mut image,
            Rgb([255u8, 255u8, 255u8]),
            0,
            0,
            scale,
            &font,
            &self.description,
        );
        draw_text_mut(
            &mut image,
            Rgb([255u8, 255u8, 255u8]),
            0,
            16,
            scale,
            &font,
            &text,
        );
        self.bytes.clear();
        let _ =
            DynamicImage::ImageRgb8(image).write_to(&mut self.bytes, image::ImageOutputFormat::Bmp);
    }
}
