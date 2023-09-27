extern crate encoding;
use crate::screens::{BasicScreen, Screen, Screenable};
use crossbeam_channel::{bounded, Receiver, Sender};
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};
use scraper::{Html, Selector};
use std::{
    rc::Rc,
    sync::{atomic::AtomicBool, atomic::Ordering, Arc},
    thread,
    time::{Duration, SystemTime},
};
use unicode_segmentation::UnicodeSegmentation;

pub struct IceScreen {
    screen: Screen,
    receiver: Receiver<IceInfo>,
    sort_x: u32,
    last_ice_info: IceInfo,
}

#[derive(Default, Clone)]
struct IceInfo {
    sorts: Vec<String>,
    images: Vec<image::DynamicImage>,
}

impl Screenable for IceScreen {
    fn get_screen(&mut self) -> &mut Screen {
        &mut self.screen
    }
}

impl BasicScreen for IceScreen {
    fn update(&mut self) {
        let ice_info = self.receiver.try_recv();
        match ice_info {
            Ok(ice_info) => {
                self.last_ice_info = ice_info;
            }
            Err(_) => {}
        }
        self.draw_screen(self.last_ice_info.clone());
        self.draw_companion_screen(self.last_ice_info.clone());
    }
}

impl IceScreen {
    fn draw_screen(&mut self, ice_info: IceInfo) {
        // draw initial image
        let mut image = RgbImage::new(256, 64);
        self.draw_ice_info(ice_info, &mut image);
        self.screen.main_screen_bytes = image.into_vec();
    }

    fn calc_next_image_x(&mut self, current_x: &mut i64, current_y: &mut i64) {
        if *current_x + 56 * 2 > 320 {
            *current_x = 5;
            *current_y += 56;
        } else {
            *current_x += 61;
        }
    }

    fn draw_companion_screen(&mut self, ice_info: IceInfo) {
        // draw initial image
        let mut image = image::DynamicImage::new_rgb8(320, 170);
        imageproc::drawing::draw_filled_rect_mut(
            &mut image,
            imageproc::rect::Rect::at(0, 0).of_size(320, 170),
            image::Rgba([255u8, 255u8, 255u8, 255]),
        );
        let mut x = -53;
        let mut y = 0;
        for ice_image in ice_info.images {
            self.calc_next_image_x(&mut x, &mut y);

            image::imageops::overlay(&mut image, &ice_image, x, y);
        }
        self.screen.companion_screen_bytes = image.as_bytes().to_vec();
    }

    fn draw_ice_info(&mut self, ice_info: IceInfo, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>) {
        let sorts = ice_info.sorts.join(" · ");
        let title_len = sorts.graphemes(true).count();
        let mut position_title = 0;
        let mut start = 0;
        if title_len * 17 < 480 {
            position_title = (1 + ((256 - (title_len as u32 * 17) / 2) / 2)) - 2;
        } else {
            start = self.sort_x as usize;

            if self.sort_x == title_len as u32 + 2u32 {
                self.sort_x = 0;
            } else {
                self.sort_x += 1;
            }
        }

        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            position_title as i32,
            24,
            Scale { x: 16.0, y: 16.0 },
            &self.screen.font,
            &rotate(&[&sorts, "   "].join(""), start),
        );
    }

    pub fn new(description: String, key: String, font: Rc<Font<'static>>) -> IceScreen {
        let (tx, rx): (Sender<IceInfo>, Receiver<IceInfo>) = bounded(1);
        let active = Arc::new(AtomicBool::new(false));
        let mut this = IceScreen {
            screen: Screen {
                description,
                key,
                font,
                active: active.clone(),
                handle: Some(thread::spawn(move || {
                    let sender = tx.to_owned();
                    let mut last_update = SystemTime::UNIX_EPOCH;
                    loop {
                        while !active.load(Ordering::Acquire) {
                            thread::park();
                        }

                        if last_update.elapsed().unwrap().as_secs() > 60 {
                            last_update = SystemTime::now();
                            let body = reqwest::blocking::get("https://eislabor.info/#beuel");
                            match body {
                                Ok(response) => {
                                    let fragment = Html::parse_fragment(&response.text().unwrap());
                                    let sorts_selector =
                                        Selector::parse(".fusion-text-14 .tlp-content a").unwrap();
                                    let sort_images_selector = Selector::parse(
                                        ".fusion-text-14 .tlp-portfolio-item a.tlp-zoom",
                                    )
                                    .unwrap();

                                    let mut images: Vec<image::DynamicImage> = Vec::new();
                                    for image in fragment.select(&sort_images_selector) {
                                        let mut buffer = Vec::new();
                                        reqwest::blocking::get(image.value().attr("href").unwrap())
                                            .unwrap()
                                            .copy_to(&mut buffer)
                                            .unwrap();
                                        let image = image::load_from_memory_with_format(
                                            &buffer,
                                            image::ImageFormat::Jpeg,
                                        )
                                        .unwrap();
                                        images.push(image.resize_exact(
                                            56,
                                            56,
                                            image::imageops::FilterType::Lanczos3,
                                        ));
                                        if images.len() == 15 {
                                            break;
                                        }
                                    }
                                    let mut sorts: Vec<String> = Vec::new();
                                    for element in fragment.select(&sorts_selector) {
                                        sorts.push(element.inner_html())
                                    }
                                    sender.send(IceInfo { sorts, images }).unwrap_or_default();
                                }
                                Err(_) => {}
                            }
                        }

                        // TODO: think about whether we want to solve this like in bitpanda screen with last_update...
                        thread::sleep(Duration::from_millis(1000));
                    }
                })),
                ..Default::default()
            },
            sort_x: 0,
            receiver: rx,
            last_ice_info: IceInfo {
                sorts: vec![String::from("Loading")],
                images: vec![],
            },
        };

        this.draw_screen(Default::default());
        this
    }
}
pub fn rotate(str: &str, count: usize) -> String {
    let mut str_vec: Vec<char> = str.chars().collect();
    str_vec.rotate_left(count);
    str_vec.iter().collect()
}
