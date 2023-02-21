extern crate winapi;
use crate::{
    config_manager::ConfigManager,
    screens::{BasicScreen, Screen, Screenable},
};
use crossbeam_channel::{bounded, Receiver, Sender};
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::{
    draw_filled_rect_mut, draw_hollow_rect_mut, draw_line_segment_mut, draw_text_mut,
};
use imageproc::rect::Rect;
use regex;
use rusttype::{Font, Scale};
use std::{
    ffi::OsStr,
    iter::once,
    os::windows::ffi::OsStrExt,
    ptr::null_mut,
    rc::Rc,
    sync::{atomic::AtomicBool, atomic::Ordering, Arc, RwLock},
    thread,
    time::{Duration, Instant},
};
use unicode_segmentation::UnicodeSegmentation;
use winapi::{
    shared::minwindef::LPARAM,
    um::{
        mmdeviceapi::*,
        winuser::{FindWindowW, *},
    },
    Interface,
};

pub struct MediaInfoScreen {
    screen: Screen,
    receiver: Receiver<MusicPlayerInfo>,
    symbols: Rc<Font<'static>>,
    title_x: u32,
    artist_x: u32,
    music_player_info: MusicPlayerInfo,
}

#[derive(Clone, Default)]
struct MusicPlayerInfo {
    playback_status: isize,
    current_track_position: isize,
    track_length: isize,
    title: String,
    artist: String,
    player_active: bool,
    system_volume: f32,
    mute: i32,
}

impl Screenable for MediaInfoScreen {
    fn get_screen(&mut self) -> &mut Screen {
        &mut self.screen
    }
}

impl BasicScreen for MediaInfoScreen {
    fn update(&mut self) {
        let music_player_info = self.receiver.try_recv();
        match music_player_info {
            Ok(music_player_info) => {
                self.draw_screen(&music_player_info);
                self.music_player_info = music_player_info;
            }
            Err(_) => {}
        }
    }
}

impl MediaInfoScreen {
    fn draw_intro(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            77,
            4,
            scale,
            &self.screen.font,
            "Media Screen",
        );
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            65,
            32,
            scale,
            &self.screen.font,
            "Winamp inactive",
        );
    }
    fn draw_artist(
        &mut self,
        artist: &String,
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
        scale: Scale,
    ) {
        let mut position_artist = 0;
        let artist_len = artist.graphemes(true).count();
        let mut start = 0usize;
        if artist_len * 17 < 480 {
            position_artist = (1 + ((256 - (artist_len as u32 * 17) / 2) / 2)) - 2;
        } else {
            start = self.artist_x as usize;
            if self.artist_x == artist_len as u32 + 2u32 {
                self.artist_x = 0;
            } else {
                self.artist_x += 1;
            }
        }

        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            position_artist as i32,
            0,
            scale,
            &self.screen.font,
            &rotate(&[&artist, "   "].join(""), Direction::Left, start),
        );
    }

    fn draw_title(
        &mut self,
        title: &String,
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
        scale: Scale,
    ) {
        let title_len = title.graphemes(true).count();
        let mut position_title = 0;
        let mut start = 0;
        if title_len * 17 < 480 {
            position_title = (1 + ((256 - (title_len as u32 * 17) / 2) / 2)) - 2;
        } else {
            start = self.title_x as usize;

            if self.title_x == title_len as u32 + 2u32 {
                self.title_x = 0;
            } else {
                self.title_x += 1;
            }
        }

        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            position_title as i32,
            16,
            scale,
            &self.screen.font,
            &rotate(&[&title, "   "].join(""), Direction::Left, start),
        );
    }

    fn draw_play_button(
        &mut self,
        playback_status: isize,
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    ) {
        let play_button = &String::from("\u{f04B}");
        let pause_button = &String::from("\u{f04C}");
        let stop_button = &String::from("\u{f04D}");

        let mut button = stop_button;
        if playback_status == 1 {
            button = play_button;
        }

        if playback_status == 3 {
            button = pause_button;
        }
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            4,
            37,
            Scale { x: 10.0, y: 10.0 },
            self.symbols.as_ref(),
            button,
        );
    }

    fn draw_elapsed(&mut self, length: isize, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>) {
        let length = length / 1000;
        let seconds = length % 60;
        let minutes = (length / 60) % 60;
        let hours = (length / 60) / 60;
        let elapsed = format!("{:0>2}:{:0>2}:{:0>2}", hours, minutes, seconds);
        let elapsed = format!("{: <12}", elapsed);
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            16,
            36,
            Scale { x: 14.0, y: 14.0 },
            &self.screen.font,
            &elapsed,
        );
    }

    fn draw_total(&mut self, length: isize, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>) {
        let seconds = length % 60;
        let minutes = (length / 60) % 60;
        let hours = (length / 60) / 60;
        let total = format!("{:0>2}:{:0>2}:{:0>2}", hours, minutes, seconds);
        let total = format!("{: >12}", total);
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            166,
            36,
            Scale { x: 14.0, y: 14.0 },
            &self.screen.font,
            &total,
        );
    }

    fn draw_elapsed_bar(
        &mut self,
        current_track_position: isize,
        track_length: isize,
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    ) {
        let indicator_position_x_min = 16.0;
        let indicator_position_x_max = 232.0;

        let progress = (current_track_position as f64 / 1000.0) / (track_length as f64);
        let position = indicator_position_x_min + (progress * indicator_position_x_max);
        draw_hollow_rect_mut(
            image,
            Rect::at(16, 50).of_size(238, 7),
            Rgb([255u8, 255u8, 255u8]),
        );

        draw_filled_rect_mut(
            image,
            Rect::at(position as i32, 50).of_size(6, 7),
            Rgb([255u8, 255u8, 255u8]),
        );

        let start = 16;
        let end = position as i32;
        let segment_length = 6;
        let line_length = (end - start) + segment_length;
        let segments = line_length / segment_length;

        for n in 0..segments {
            let formula = start as f32 + (n as f32 * segment_length as f32);
            draw_line_segment_mut(
                image,
                (formula, 53.0),
                (formula + (segment_length / 2) as f32, 53.0),
                Rgb([255u8, 255u8, 255u8]),
            );
        }
    }

    fn draw_mute_speaker(&mut self, mute: i32, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>) {
        let mute_speaker = &String::from("\u{f6a9}");
        if mute == 1 {
            draw_text_mut(
                image,
                Rgb([255u8, 255u8, 255u8]),
                118,
                38,
                Scale { x: 10.0, y: 10.0 },
                self.symbols.as_ref(),
                mute_speaker,
            );
        }
    }
    fn draw_volume_bar(
        &mut self,
        system_volume: f32,
        playback_status: isize,
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    ) {
        let progress = (1.0 + (238.0 * system_volume)) as u32;

        draw_hollow_rect_mut(
            image,
            Rect::at(16, 50).of_size(238, 6),
            Rgb([255u8, 255u8, 255u8]),
        );
        let small_speaker = &String::from("\u{f027}");
        let big_speaker = &String::from("\u{f028}");

        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            16,
            38,
            Scale { x: 10.0, y: 10.0 },
            self.symbols.as_ref(),
            small_speaker,
        );

        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            240,
            37,
            Scale { x: 10.0, y: 10.0 },
            self.symbols.as_ref(),
            big_speaker,
        );
        draw_filled_rect_mut(
            image,
            Rect::at(16, 50).of_size(progress, 6),
            Rgb([255u8, 255u8, 255u8]),
        );

        self.draw_play_button(playback_status, image);
    }

    fn draw_screen(&mut self, music_player_info: &MusicPlayerInfo) {
        let mut image = RgbImage::new(256, 64);
        let scale = Scale { x: 16.0, y: 16.0 };
        let seconds = Duration::from_secs(3);
        if self.screen.mode_timeout.unwrap_or(Instant::now()).elapsed() >= seconds {
            self.screen.mode = 0;
        }

        if (music_player_info.artist != self.music_player_info.artist)
            || (music_player_info.title != self.music_player_info.title)
        {
            self.title_x = 0;
        }

        if music_player_info.artist != self.music_player_info.artist {
            self.artist_x = 0;
        }

        if music_player_info.player_active {
            self.draw_artist(&music_player_info.artist, &mut image, scale);
            self.draw_title(&music_player_info.title, &mut image, scale);
            self.draw_mute_speaker(music_player_info.mute, &mut image);

            if self.screen.mode == 0 {
                self.draw_play_button(music_player_info.playback_status, &mut image);
                self.draw_elapsed(music_player_info.current_track_position, &mut image);
                self.draw_total(music_player_info.track_length, &mut image);
                self.draw_elapsed_bar(
                    music_player_info.current_track_position,
                    music_player_info.track_length,
                    &mut image,
                );
            } else {
                // DRAW VOLUME BAR
                self.draw_volume_bar(
                    music_player_info.system_volume,
                    music_player_info.playback_status,
                    &mut image,
                );
            }
        } else {
            self.draw_intro(&mut image, scale);
        }
        self.screen.bytes = image.into_vec();
    }

    pub fn new(
        description: String,
        key: String,
        font: Rc<Font<'static>>,
        symbols: Rc<Font<'static>>,
        config_manager: Arc<RwLock<ConfigManager>>,
    ) -> MediaInfoScreen {
        let (tx, rx): (Sender<MusicPlayerInfo>, Receiver<MusicPlayerInfo>) = bounded(1);
        let active = Arc::new(AtomicBool::new(false));

        let mut this = MediaInfoScreen {
            screen: Screen {
                description,
                font,
                config_manager,
                key,
                active: active.clone(),
                handle: Some(thread::spawn(move || {
                    let window: Vec<u16> = OsStr::new("Winamp v1.x")
                        .encode_wide()
                        .chain(once(0))
                        .collect();
                    let sender = tx.to_owned();
                    let active = active.clone();
                    let match_correct_artist_and_title_format;
                    let match_artist_and_title;
                    let match_artist_or_title;
                    match regex::Regex::new(r"\s(.*)-") {
                        Ok(regex) => {
                            match_correct_artist_and_title_format = regex;
                        }
                        Err(err) => {
                            eprintln!("REGEX ERROR: {:?}", err);
                            return;
                        }
                    }
                    match regex::Regex::new(r"(.*) - (.*)") {
                        Ok(regex) => {
                            match_artist_and_title = regex;
                        }
                        Err(err) => {
                            eprintln!("REGEX ERROR: {:?}", err);
                            return;
                        }
                    }

                    match regex::Regex::new(r"\s(.*?)\s") {
                        Ok(regex) => {
                            match_artist_or_title = regex;
                        }
                        Err(err) => {
                            eprintln!("REGEX ERROR: {:?}", err);
                            return;
                        }
                    }

                    unsafe {
                        winapi::um::objbase::CoInitialize(std::ptr::null_mut());
                    }

                    loop {
                        while !active.load(Ordering::Acquire) {
                            thread::park();
                        }
                        let mut music_player_info: MusicPlayerInfo = Default::default();
                        let hwnd = unsafe { FindWindowW(window.as_ptr(), null_mut()) };
                        if hwnd != null_mut() {
                            unsafe {
                                // 1 == playing, 3 == paused, anything else == stopped
                                let playback_status = SendMessageW(hwnd, WM_USER, 0, 104);
                                music_player_info.playback_status = playback_status;
                                // current position in msecs
                                let mut current_track_position =
                                    SendMessageW(hwnd, WM_USER, 0, 105);
                                if playback_status != 1 && playback_status != 3 {
                                    current_track_position = 0;
                                }

                                music_player_info.current_track_position = current_track_position;

                                // track length in seconds (multiply by thousand)
                                let track_length = SendMessageW(hwnd, WM_USER, 1, 105);
                                music_player_info.track_length = track_length;
                                // get title
                                let current_index = SendMessageW(hwnd, WM_USER, 0, 125);
                                let title_length =
                                    SendMessageW(hwnd, WM_GETTEXTLENGTH, current_index as usize, 0);

                                // WINAMP VOLUME. NOT USED RIGHT NOW.
                                // let mut volume = SendMessageW(hwnd, WM_USER, -666i32 as usize, 122);

                                let buffer_length = title_length + 1;
                                let mut buffer = Vec::<u16>::with_capacity(buffer_length as usize);
                                buffer.resize(buffer_length as usize, 0);
                                SendMessageW(
                                    hwnd,
                                    WM_GETTEXT,
                                    buffer_length as usize,
                                    buffer.as_mut_ptr() as LPARAM,
                                );
                                let data = String::from_utf16_lossy(&buffer);

                                music_player_info.player_active = title_length > 0
                                    && match_correct_artist_and_title_format.is_match(&data);

                                let caps = match_correct_artist_and_title_format.captures(&data);
                                match caps {
                                    Some(caps) => {
                                        let artist_and_title =
                                            caps.get(1).map_or("", |m| m.as_str()).trim();

                                        let artist_and_title_caps =
                                            match_artist_and_title.captures(&artist_and_title);
                                        match artist_and_title_caps {
                                            Some(artist_and_title_caps) => {
                                                let artist = artist_and_title_caps
                                                    .get(1)
                                                    .map_or("", |m| m.as_str())
                                                    .trim();
                                                let title = artist_and_title_caps
                                                    .get(2)
                                                    .map_or("", |m| m.as_str())
                                                    .trim();

                                                music_player_info.artist = artist.to_string();
                                                music_player_info.title = title.to_string();
                                                drop(artist);
                                                drop(title);
                                            }
                                            None => {
                                                // check, if only artist OR title are there
                                                let artist_or_title_caps =
                                                    match_artist_or_title.captures(&data);

                                                match artist_or_title_caps {
                                                    Some(artist_or_title_caps) => {
                                                        let title = artist_or_title_caps
                                                            .get(1)
                                                            .map_or("", |m| m.as_str())
                                                            .trim();
                                                        music_player_info.title = title.to_string();
                                                        drop(title);
                                                        music_player_info.artist = "".into();
                                                    }
                                                    None => {
                                                        music_player_info.player_active = false;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    None => {
                                        music_player_info.player_active = false;
                                    }
                                }
                            }
                        } else {
                            music_player_info.player_active = false;
                        }

                        let volume_data = get_master_volume();
                        music_player_info.system_volume = volume_data.0;
                        music_player_info.mute = volume_data.1;
                        sender.try_send(music_player_info).unwrap_or_default();
                        thread::sleep(Duration::from_millis(200));
                    }
                })),
                ..Default::default()
            },
            music_player_info: Default::default(),
            symbols: Rc::clone(&symbols),
            title_x: 0,
            artist_x: 0,
            receiver: rx,
        };
        this.draw_screen(&Default::default());
        this
    }
}
// MOVE EVERYTHING BELOW SOMEWHERE ELSE, TO SOME MODULE ETC.
pub enum Direction {
    Left,
    //Right,
}

pub fn rotate(str: &str, direction: Direction, count: usize) -> String {
    let mut str_vec: Vec<char> = str.chars().collect();
    match direction {
        Direction::Left => str_vec.rotate_left(count),
        //Direction::Right => str_vec.rotate_right(count),
    }
    str_vec.iter().collect()
}

// TODO: disable for non-windows OS and find another way.
pub fn get_master_volume() -> (f32, i32) {
    let mut current_volume = 0.0 as f32;
    let mut mute = 0;
    unsafe {
        let mut device_enumerator: *mut winapi::um::mmdeviceapi::IMMDeviceEnumerator =
            std::ptr::null_mut();
        winapi::um::combaseapi::CoCreateInstance(
            &winapi::um::mmdeviceapi::CLSID_MMDeviceEnumerator,
            std::ptr::null_mut(),
            winapi::um::combaseapi::CLSCTX_ALL,
            &IMMDeviceEnumerator::uuidof(),
            &mut device_enumerator as *mut *mut winapi::um::mmdeviceapi::IMMDeviceEnumerator
                as *mut _,
        );
        let mut default_device: *mut winapi::um::mmdeviceapi::IMMDevice = std::mem::zeroed();
        (*device_enumerator).GetDefaultAudioEndpoint(
            winapi::um::mmdeviceapi::eRender,
            winapi::um::mmdeviceapi::eConsole,
            &mut default_device,
        );
        if default_device == std::ptr::null_mut() {
            return (0.0, 0);
        }

        (*device_enumerator).Release();
        let mut endpoint_volume: *mut winapi::um::endpointvolume::IAudioEndpointVolume =
            std::mem::zeroed();

        (*default_device).Activate(
            &winapi::um::endpointvolume::IAudioEndpointVolume::uuidof(),
            winapi::shared::wtypesbase::CLSCTX_INPROC_SERVER,
            std::ptr::null_mut(),
            &mut endpoint_volume as *mut *mut winapi::um::endpointvolume::IAudioEndpointVolume
                as *mut _,
        );

        (*default_device).Release();
        (*endpoint_volume).GetMasterVolumeLevelScalar(&mut current_volume as *mut f32);
        (*endpoint_volume).GetMute(&mut mute as *mut i32);
    }
    (current_volume, mute)
}

impl Drop for MediaInfoScreen {
    fn drop(&mut self) {
        unsafe {
            winapi::um::combaseapi::CoUninitialize();
        }
    }
}
