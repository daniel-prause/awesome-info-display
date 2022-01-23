use crate::config_manager::ConfigManager;
use crate::screens::BasicScreen;
use crate::screens::Screen;
use crate::screens::ScreenControl;

use chrono::{DateTime, Local}; // 0.4.15
use error_chain::error_chain;
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::draw_text_mut;
use rusttype::Font;
use rusttype::Scale;
use std::fmt::Debug;

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
#[derive(Debug)]
pub struct BitpandaScreen {
    screen: Screen,
    wallet_value: Mutex<f64>,
    last_update: Mutex<SystemTime>,
    initial_tryout: AtomicBool,
}

impl BasicScreen for std::sync::Arc<RwLock<BitpandaScreen>> {
    fn description(&self) -> String {
        self.read().unwrap().screen.description.clone()
    }

    fn current_image(&self) -> Vec<u8> {
        self.read().unwrap().screen.current_image()
    }

    fn update(&mut self) {
        BitpandaScreen::update(self.clone());
    }

    fn start(&self) {
        self.read().unwrap().screen.start_worker();
    }

    fn stop(&self) {
        self.read().unwrap().screen.stop_worker();
    }

    fn key(&self) -> String {
        self.read().unwrap().screen.key()
    }

    fn initial_update_called(&mut self) -> bool {
        self.write().unwrap().screen.initial_update_called()
    }

    fn enabled(&self) -> bool {
        self.read()
            .unwrap()
            .screen
            .config_manager
            .read()
            .unwrap()
            .config
            .bitpanda_screen_active
    }

    fn set_status(&self, status: bool) {
        self.read()
            .unwrap()
            .screen
            .config_manager
            .write()
            .unwrap()
            .config
            .bitpanda_screen_active = status;
    }
}

impl BitpandaScreen {
    pub fn draw_wallet_value(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
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
            &format!("{: >10}€", self.wallet_value.lock().unwrap()),
        );
    }

    pub fn draw_updated_at(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        let date_value: DateTime<Local> = self.last_update.lock().unwrap().clone().into();
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

    fn update(instance: Arc<RwLock<BitpandaScreen>>) {
        let mut image = RgbImage::new(256, 64);
        let scale = Scale { x: 16.0, y: 16.0 };

        instance
            .write()
            .unwrap()
            .draw_wallet_value(&mut image, scale);
        instance.write().unwrap().draw_updated_at(&mut image, scale);
        *instance.write().unwrap().screen.bytes.lock().unwrap() = image.into_vec();
    }

    pub fn new(
        description: String,
        key: String,
        font: Arc<Mutex<Option<Font<'static>>>>,
        config_manager: Arc<RwLock<ConfigManager>>,
    ) -> Arc<RwLock<BitpandaScreen>> {
        let this = Arc::new(RwLock::new(BitpandaScreen {
            screen: Screen {
                description,
                font,
                config_manager,
                key,
                ..Default::default()
            },
            wallet_value: Mutex::new(0.0),
            last_update: Mutex::new(SystemTime::now()),
            initial_tryout: AtomicBool::new(false),
        }));
        let builder = thread::Builder::new().name("JOB_EXECUTOR".into());

        *this.read().unwrap().screen.handle.lock().unwrap() = Some(
            builder
                .spawn({
                    let this = this.clone();
                    move || loop {
                        while !this.read().unwrap().screen.active.load(Ordering::Acquire) {
                            thread::park();
                        }
                        thread::sleep(Duration::from_millis(1000));
                        let value = *this.read().unwrap().wallet_value.lock().unwrap();
                        let elapsed = *this.read().unwrap().last_update.lock().unwrap();
                        match elapsed.elapsed() {
                            Ok(duration) => {
                                if (duration.as_secs() > 60
                                    || duration.as_secs() < 60
                                        && !this
                                            .read()
                                            .unwrap()
                                            .initial_tryout
                                            .load(Ordering::Acquire))
                                    && this
                                        .read()
                                        .unwrap()
                                        .screen
                                        .config_manager
                                        .read()
                                        .unwrap()
                                        .config
                                        .bitpanda_api_key
                                        .clone()
                                        != ""
                                {
                                    // unlock value mutex until request is done
                                    this.read()
                                        .unwrap()
                                        .initial_tryout
                                        .store(true, Ordering::Release);
                                    drop(value);
                                    drop(elapsed);
                                    // 1. get current values for crypto coins
                                    let body = reqwest::blocking::get(
                                        "https://api.bitpanda.com/v1/ticker",
                                    );
                                    match body {
                                        Ok(text) => match text.text() {
                                            Ok(asset_values) => {
                                                // 2. get wallet values
                                                let client = reqwest::blocking::Client::new();
                                                let wallet_values = client
                                                    .get("https://api.bitpanda.com/v1/wallets")
                                                    .header(
                                                        "X-API-KEY",
                                                        this.read()
                                                            .unwrap()
                                                            .screen
                                                            .config_manager
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
                                                                            &wallet_json["data"]
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
                                                                            let amount_of_eur =
                                                                                assets[asset_key]
                                                                                    ["EUR"]
                                                                                    .as_str()
                                                                                    .unwrap()
                                                                                    .parse::<f64>()
                                                                                    .unwrap();
                                                                            let amount_of_crypto =
                                                                                wallet
                                                                                    ["attributes"]
                                                                                    ["balance"]
                                                                                    .as_str()
                                                                                    .unwrap()
                                                                                    .parse::<f64>()
                                                                                    .unwrap();

                                                                            sum += amount_of_crypto
                                                                                * amount_of_eur;
                                                                        }
                                                                    }
                                                                    *this
                                                                        .read()
                                                                        .unwrap()
                                                                        .wallet_value
                                                                        .lock()
                                                                        .unwrap() = (sum * 100.0)
                                                                        .round()
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
                                        },
                                        Err(e) => {
                                            println!("Error: {:?}", e);
                                        }
                                    }
                                    *this.read().unwrap().last_update.lock().unwrap() =
                                        SystemTime::now();
                                }
                            }
                            Err(e) => {
                                println!("Error: {:?}", e);
                            }
                        }
                    }
                })
                .expect("Cannot create JOB_EXECUTOR thread"),
        );
        this.clone()
    }
}
