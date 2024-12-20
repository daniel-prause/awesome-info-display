use crate::config_manager::ConfigManager;
use crate::screens::{BasicScreen, Screen, Screenable};
use crate::DEVICES;
use ab_glyph::{FontArc, PxScale};
use exchange_format::*;
use image::{EncodableLayout, GenericImage, ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::draw_text_mut;
use libloading::Library;

use std::ffi::CString;
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, Arc, RwLock};

struct Lib {
    library: Library,
}

impl Lib {
    fn new(path_buf: PathBuf) -> Lib {
        Lib {
            library: unsafe { libloading::Library::new(path_buf).expect("Failed to load library") },
        }
    }

    fn get_key(&self) -> String {
        let get_key: libloading::Symbol<unsafe extern "C" fn() -> *mut i8> =
            unsafe { self.library.get(b"get_key").expect("Get key not found!") };
        unsafe {
            CString::from_raw(get_key())
                .to_owned()
                .to_string_lossy()
                .to_string()
        }
    }

    fn get_description(&self) -> String {
        let get_description: libloading::Symbol<unsafe extern "C" fn() -> *mut i8> = unsafe {
            self.library
                .get(b"get_description")
                .expect("Get key not found!")
        };
        unsafe {
            CString::from_raw(get_description())
                .to_owned()
                .to_string_lossy()
                .to_string()
        }
    }

    fn get_config_layout(&self) -> ExchangeableConfig {
        let get_config_layout: std::result::Result<
            libloading::Symbol<unsafe extern "C" fn() -> *mut i8>,
            libloading::Error,
        > = unsafe { self.library.get(b"get_config_layout") };

        match get_config_layout {
            Ok(config) => ExchangeableConfig::from(unsafe {
                CString::from_raw(config())
                    .to_owned()
                    .to_string_lossy()
                    .to_string()
            }),
            Err(_) => ExchangeableConfig::default(),
        }
    }

    // TODO: maybe give this method parameters of which screen should be drawn
    fn get_screen(&self, device: &String) -> Option<ExchangeFormat> {
        match unsafe {
            self.library
                .get::<libloading::Symbol<unsafe extern "C" fn(&String) -> *mut i8>>(b"get_screen")
        } {
            Ok(get_screen) => {
                let ptr = unsafe { get_screen(device) };

                if ptr == std::ptr::null_mut() {
                    return None;
                }

                return unsafe {
                    Some(
                        serde_json::from_str(
                            CString::from_raw(ptr)
                                .to_owned()
                                .to_string_lossy()
                                .to_string()
                                .as_str(),
                        )
                        .unwrap_or_default(),
                    )
                };
            }
            Err(_) => return None,
        }
    }

    fn set_current_config(&self, config: *mut i8) {
        if let Ok(set_current_config) = unsafe {
            self.library
                .get::<libloading::Symbol<unsafe extern "C" fn(*mut i8)>>(b"set_current_config")
        } {
            unsafe { set_current_config(config) }
        }
    }
}

pub struct PluginScreen {
    screen: Screen,
    lib: Arc<Lib>,
}

impl Screenable for PluginScreen {
    fn get_screen(&mut self) -> &mut Screen {
        &mut self.screen
    }
}

impl BasicScreen for PluginScreen {
    fn update(&mut self) {
        for (key, device) in DEVICES.iter() {
            match self.lib.clone().get_screen(key) {
                Some(screen) => {
                    self.draw_for_device(
                        key,
                        device.screen_height(),
                        device.screen_width(),
                        screen,
                    );
                }
                None => {}
            }
        }
    }

    fn set_current_config(&mut self, config: ExchangeableConfig) {
        self.lib.clone().set_current_config(config.to_raw());
    }
}

// TODO: think about multiple exchange formats for different devices
impl PluginScreen {
    fn draw_for_device(
        &mut self,
        device: &String,
        height: u32,
        width: u32,
        exchange_format: ExchangeFormat,
    ) {
        let mut image = RgbImage::new(width, height);
        self.draw_exchange_format(&mut image, exchange_format);
        *self.screen.device_screen_bytes.get_mut(device).unwrap() = image.into_vec();
    }

    pub fn draw_exchange_format(
        &mut self,
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
        exchange_format: ExchangeFormat,
    ) {
        for item in exchange_format.items.iter() {
            match item {
                Item::Text(text) => {
                    // determine color
                    let color;
                    if text.color.len() == 3 {
                        color = Rgb([text.color[0], text.color[1], text.color[2]]);
                    } else {
                        color = Rgb([255, 255, 255])
                    }

                    // determine font
                    let font = if text.symbol {
                        &self.screen.symbols
                    } else {
                        &self.screen.font
                    };

                    // draw text
                    draw_text_mut(
                        image,
                        color,
                        text.x,
                        text.y,
                        PxScale {
                            x: text.scale_x,
                            y: text.scale_y,
                        },
                        font,
                        &text.value,
                    );
                }
                Item::Image(overlay_image) => {
                    let mut overlay = RgbImage::new(overlay_image.width, overlay_image.height);
                    overlay.copy_from_slice(overlay_image.value.as_bytes());
                    image
                        .copy_from(&overlay, overlay_image.x, overlay_image.y)
                        .unwrap_or_default();
                }
            }
        }
    }

    pub fn new(
        font: FontArc,
        symbols: FontArc,
        config_manager: Arc<RwLock<ConfigManager>>,
        library_path: PathBuf,
    ) -> PluginScreen {
        let active = Arc::new(AtomicBool::new(false));

        // load library
        let lib = Arc::new(Lib::new(library_path.clone()));
        let mut this = PluginScreen {
            lib: lib.clone(),
            screen: Screen {
                description: lib.clone().get_description(),
                key: lib.clone().get_key(),
                config_layout: lib.clone().get_config_layout(),
                font,
                symbols,
                config_manager: config_manager.clone(),
                active: active.clone(),
                // get rid of this..
                handle: None,
                ..Default::default()
            },
        };
        {
            // set initial config
            let this = &mut this;
            let config = this
                .screen
                .config_manager
                .read()
                .unwrap()
                .get_screen_config(&lib.clone().get_key())
                .unwrap_or_default()
                .to_raw();
            this.lib.clone().set_current_config(config);
        };
        for (key, device) in DEVICES.iter() {
            this.draw_for_device(
                &key,
                device.screen_height(),
                device.screen_width(),
                ExchangeFormat::default(),
            );
        }
        this
    }
}
