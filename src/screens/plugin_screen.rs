use crate::config_manager::ConfigManager;
use crate::screens::{BasicScreen, Screen, Screenable};
use image::RgbImage;
use rusttype::Font;
use std::ffi::CString;
use std::path::PathBuf;
use std::{
    rc::Rc,
    sync::{atomic::AtomicBool, atomic::Ordering, Arc, RwLock},
    thread,
    time::Duration,
};

pub struct PluginScreen {
    screen: Screen,
    //receiver: Receiver<ClockInfo>,
}

impl Screenable for PluginScreen {
    fn get_screen(&mut self) -> &mut Screen {
        &mut self.screen
    }
}

impl BasicScreen for PluginScreen {
    fn update(&mut self) {
        self.draw_screen();
        /*
        let clock_info = self.receiver.try_recv();
        match clock_info {
            Ok(clock_info_state) => {
                self.draw_screen(clock_info_state.local);
            }
            Err(_) => {}
        }
         */
    }
}

impl PluginScreen {
    fn draw_screen(&mut self) {
        // draw initial image
        let image = RgbImage::new(256, 64);
        //let scale = Scale { x: 16.0, y: 16.0 };

        //self.draw_clock(&mut image, local, scale);
        self.screen.main_screen_bytes = image.into_vec();
    }

    /*
       pub fn draw_clock(
           &mut self,
           image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
           local: DateTime<Local>,
           scale: Scale,
       ) {
           let date_time = local.format("%d.%m.%Y %H:%M:%S").to_string();
           draw_text_mut(
               image,
               Rgb([255u8, 255u8, 255u8]),
               46,
               24,
               scale,
               &self.screen.font,
               &date_time,
           );
       }
    */
    pub fn new(
        font: Rc<Font<'static>>,
        config_manager: Arc<RwLock<ConfigManager>>,
        library_path: PathBuf,
    ) -> PluginScreen {
        //let (tx, rx): (Sender<ClockInfo>, Receiver<ClockInfo>) = bounded(1);
        // load library:

        let active = Arc::new(AtomicBool::new(false));
        // load library
        let get_key: libloading::Symbol<unsafe extern "C" fn() -> *mut i8>;
        let get_description: libloading::Symbol<unsafe extern "C" fn() -> *mut i8>;
        let lib: Arc<libloading::Library>;
        unsafe {
            lib = Arc::new(libloading::Library::new(library_path).expect("Failed to load library"));
            get_key = lib.get(b"get_key").expect("Get key not found!");
            get_description = lib.get(b"get_description").expect("Get key not found!");
        }
        let this = PluginScreen {
            screen: Screen {
                description: unsafe {
                    CString::from_raw(get_description())
                        .to_owned()
                        .to_string_lossy()
                        .to_string()
                },
                key: unsafe {
                    CString::from_raw(get_key())
                        .to_owned()
                        .to_string_lossy()
                        .to_string()
                },
                font,
                active: active.clone(),
                handle: Some(thread::spawn(move || {
                    //let sender = tx.to_owned();
                    let active = active;
                    let lib: Arc<libloading::Library> = lib.clone();
                    loop {
                        while !active.load(Ordering::Acquire) {
                            thread::park();
                        }
                        // TODO: draw screen!
                        unsafe {
                            let get_key: libloading::Symbol<unsafe extern "C" fn() -> *mut i8> =
                                lib.get(b"get_key").expect("Function gone :(");
                            println!(
                                "KEY: {}",
                                CString::from_raw(get_key())
                                    .to_owned()
                                    .to_string_lossy()
                                    .to_string()
                            )
                        }
                        //let clock_info: ClockInfo = ClockInfo::default();
                        //sender.try_send(clock_info).unwrap_or_default();
                        thread::sleep(Duration::from_millis(1000));
                    }
                })),
                config_manager,
                ..Default::default()
            },
        };

        //let initial_clock_info = ClockInfo::default();
        //this.draw_screen(initial_clock_info.local);
        this
    }
}
