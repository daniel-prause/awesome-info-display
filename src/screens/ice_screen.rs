extern crate encoding;
use crate::{
    config_manager::ConfigManager,
    screens::{BasicScreen, Screen, Screenable},
};
use crossbeam_channel::{bounded, Receiver, Sender};
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};
use scraper::{Html, Selector};
use std::{
    rc::Rc,
    sync::{atomic::AtomicBool, atomic::Ordering, Arc, RwLock},
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
                self.last_ice_info = ice_info.clone();
            }
            Err(_) => {}
        }
        self.draw_screen(self.last_ice_info.clone());
    }
}

impl IceScreen {
    fn draw_screen(&mut self, ice_info: IceInfo) {
        // draw initial image
        let mut image = RgbImage::new(256, 64);
        self.draw_ice_info(ice_info, &mut image);
        self.screen.bytes = image.into_vec();
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

    pub fn new(
        description: String,
        key: String,
        font: Rc<Font<'static>>,
        config_manager: Arc<RwLock<ConfigManager>>,
    ) -> IceScreen {
        let (tx, rx): (Sender<IceInfo>, Receiver<IceInfo>) = bounded(1);
        let active = Arc::new(AtomicBool::new(false));
        let mut this = IceScreen {
            screen: Screen {
                description,
                key,
                font,
                config_manager: config_manager.clone(),
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
                                    let selector =
                                        Selector::parse(".fusion-text-14 .tlp-content a").unwrap();

                                    let mut sorts: Vec<String> = Vec::new();
                                    for element in fragment.select(&selector) {
                                        sorts.push(element.inner_html())
                                    }
                                    sender.send(IceInfo { sorts }).unwrap_or_default();
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
