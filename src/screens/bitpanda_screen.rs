use crate::config_manager::ConfigManager;
use crate::screens::BasicScreen;
use crate::screens::Screen;
use crate::screens::ScreenControl;
use chrono::{DateTime, Local};
use crossbeam_channel::bounded;
use crossbeam_channel::{Receiver, Sender};
use error_chain::error_chain;
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::draw_text_mut;
use rusttype::Font;
use rusttype::Scale;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

use serde_json::Value;

error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
    }
}

pub struct BitpandaScreen {
    screen: Screen,
    receiver: Receiver<WalletInfo>,
}

struct WalletInfo {
    wallet_value: f64,
    last_update: SystemTime,
}

impl Default for WalletInfo {
    fn default() -> WalletInfo {
        WalletInfo {
            wallet_value: 0f64,
            last_update: SystemTime::now(),
        }
    }
}

impl BasicScreen for BitpandaScreen {
    fn description(&self) -> String {
        self.screen.description.clone()
    }

    fn current_image(&self) -> Vec<u8> {
        self.screen.current_image()
    }

    fn update(&mut self) {
        BitpandaScreen::update(self);
    }

    fn start(&self) {
        self.screen.start_worker();
    }

    fn stop(&self) {
        self.screen.stop_worker();
    }

    fn key(&self) -> String {
        self.screen.key()
    }

    fn initial_update_called(&mut self) -> bool {
        self.screen.initial_update_called()
    }

    fn enabled(&self) -> bool {
        self.screen
            .config_manager
            .read()
            .unwrap()
            .config
            .bitpanda_screen_active
    }

    fn set_status(&self, status: bool) {
        self.screen
            .config_manager
            .write()
            .unwrap()
            .config
            .bitpanda_screen_active = status;
    }
}

impl BitpandaScreen {
    fn draw_screen(&mut self, wallet_info: WalletInfo) {
        // draw initial image
        let mut image = RgbImage::new(256, 64);
        let scale = Scale { x: 16.0, y: 16.0 };

        self.draw_wallet_value(wallet_info.wallet_value, &mut image, scale);
        self.draw_updated_at(wallet_info.last_update, &mut image, scale);
        self.screen.bytes = image.into_vec();
    }
    pub fn draw_wallet_value(
        &mut self,
        wallet_value: f64,
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
        scale: Scale,
    ) {
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            0,
            0,
            scale,
            self.screen.font.lock().unwrap().as_ref().unwrap(),
            "Bitpanda",
        );
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            160,
            0,
            scale,
            self.screen.font.lock().unwrap().as_ref().unwrap(),
            &format!("{: >10}€", wallet_value),
        );
    }

    pub fn draw_updated_at(
        &mut self,
        last_update: SystemTime,
        image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
        scale: Scale,
    ) {
        let date_value: DateTime<Local> = last_update.into();
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            84,
            20,
            scale,
            self.screen.font.lock().unwrap().as_ref().unwrap(),
            "Last update",
        );

        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            52,
            40,
            scale,
            self.screen.font.lock().unwrap().as_ref().unwrap(),
            &date_value.format("%d.%m.%Y %T").to_string(),
        );
    }

    fn update(&mut self) {
        let wallet_info = self.receiver.try_recv();
        match wallet_info {
            Ok(wallet_info) => {
                self.draw_screen(wallet_info);
            }
            Err(_) => {}
        }
    }

    pub fn new(
        description: String,
        key: String,
        font: Arc<Mutex<Option<Font<'static>>>>,
        config_manager: Arc<RwLock<ConfigManager>>,
    ) -> BitpandaScreen {
        let (tx, rx): (Sender<WalletInfo>, Receiver<WalletInfo>) = bounded(1);
        let active = Arc::new(AtomicBool::new(false));
        let mut this = BitpandaScreen {
            screen: Screen {
                description,
                font,
                config_manager: config_manager.clone(),
                key,
                active: active.clone(),
                handle: Some(thread::spawn(move || {
                    let mut initial_tryout = false;
                    let mut wallet_value = 0.0;
                    let mut last_update = SystemTime::now();
                    let sender = tx.to_owned();
                    let active = active.clone();
                    loop {
                        while !active.load(Ordering::Acquire) {
                            thread::park();
                        }

                        thread::sleep(Duration::from_millis(1000));
                        match last_update.elapsed() {
                            Ok(duration) => {
                                if (duration.as_secs() > 60
                                    || duration.as_secs() < 60 && !initial_tryout)
                                    && config_manager
                                        .read()
                                        .unwrap()
                                        .config
                                        .bitpanda_api_key
                                        .clone()
                                        != ""
                                {
                                    initial_tryout = true;
                                    // 1. get current values for crypto coins
                                    let body = reqwest::blocking::get(
                                        "https://api.bitpanda.com/v1/ticker",
                                    );
                                    match body {
                                        Ok(text) => {
                                            match text.text() {
                                                Ok(asset_values) => {
                                                    // 2. get wallet values
                                                    let client = reqwest::blocking::Client::new();
                                                    let wallet_values = client
                                                        .get("https://api.bitpanda.com/v1/wallets")
                                                        .header(
                                                            "X-API-KEY",
                                                            config_manager
                                                                .read()
                                                                .unwrap()
                                                                .config
                                                                .bitpanda_api_key
                                                                .clone(),
                                                        )
                                                        .send();
                                                    match wallet_values {
                                                        Ok(res) => match res.text() {
                                                            Ok(wallet_response) => {
                                                                let try_wallet_json =
                                                                    serde_json::from_str(
                                                                        &wallet_response,
                                                                    );
                                                                match try_wallet_json {
                                                                    Ok(wallet_json) => {
                                                                        let wallet_json: Value =
                                                                            wallet_json;
                                                                        let wallets: Vec<Value> =
                                                                            serde_json::from_str(
                                                                                &wallet_json
                                                                                    ["data"]
                                                                                    .to_string(),
                                                                            )
                                                                            .unwrap_or_default();
                                                                        let assets: Value =
                                                                            serde_json::from_str(
                                                                                &asset_values
                                                                                    .to_string(),
                                                                            )
                                                                            .unwrap_or_default();
                                                                        let mut sum = 0.0;
                                                                        for wallet in wallets {
                                                                            let asset_key = wallet
                                                                    ["attributes"]
                                                                    ["cryptocoin_symbol"]
                                                                    .as_str()
                                                                    .unwrap();
                                                                            if wallet["attributes"]
                                                                                ["balance"]
                                                                                != "0.00000000"
                                                                            {
                                                                                let amount_of_eur = assets
                                                                        [asset_key]["EUR"]
                                                                        .as_str()
                                                                        .unwrap()
                                                                        .parse::<f64>()
                                                                        .unwrap();
                                                                                let amount_of_crypto = wallet
                                                                        ["attributes"]["balance"]
                                                                        .as_str()
                                                                        .unwrap()
                                                                        .parse::<f64>()
                                                                        .unwrap();

                                                                                sum += amount_of_crypto
                                                                        * amount_of_eur;
                                                                            }
                                                                        }

                                                                        wallet_value =
                                                                            (sum * 100.0).round()
                                                                                / 100.0;
                                                                    }
                                                                    Err(e) => {
                                                                        println!("Error: {:?}", e);
                                                                    }
                                                                }
                                                            }
                                                            Err(e) => {
                                                                println!("Error: {:?}", e);
                                                            }
                                                        },
                                                        Err(e) => {
                                                            println!("Error: {:?}", e);
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    println!("Error: {:?}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            println!("Error: {:?}", e);
                                        }
                                    }
                                    last_update = SystemTime::now();
                                    let wallet_info: WalletInfo = WalletInfo {
                                        wallet_value,
                                        last_update,
                                    };
                                    sender.try_send(wallet_info).unwrap_or_default();
                                }
                            }
                            Err(e) => {
                                println!("Error: {:?}", e);
                            }
                        }
                    }
                })),
                ..Default::default()
            },
            receiver: rx,
        };

        this.draw_screen(Default::default());
        this
    }
}
