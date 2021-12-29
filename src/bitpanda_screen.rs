use crate::config_manager::ConfigManager;
use crate::screen::Screen;
use crate::screen::SpecificScreen;

use chrono::{DateTime, Local}; // 0.4.15
use error_chain::error_chain;
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::draw_text_mut;
use rusttype::Font;
use rusttype::Scale;
use std::fmt::Debug;

use std::sync::{atomic::Ordering, Arc, Mutex, RwLock};
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
#[derive(Debug, Clone)]
pub struct BitpandaScreen {
    screen: Screen,
    wallet_value: Arc<Mutex<f64>>,
    last_update: Arc<Mutex<SystemTime>>,
}

impl SpecificScreen for BitpandaScreen {
    fn description(&self) -> &String {
        &self.screen.description
    }

    fn current_image(&self) -> Vec<u8> {
        self.screen.bytes.clone()
    }

    fn update(&mut self) {
        BitpandaScreen::update(self);
    }

    fn start(&self) {
        self.screen.active.store(true, Ordering::Release);
        if !self.screen.handle.lock().unwrap().is_none() {
            self.screen
                .handle
                .lock()
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .thread()
                .unpark();
        }
    }

    fn stop(&self) {
        self.screen.active.store(false, Ordering::Release);
    }

    fn initial_update_called(&mut self) -> bool {
        if !self.screen.initial_update_called.load(Ordering::Acquire) {
            self.screen
                .initial_update_called
                .store(true, Ordering::Release);
            return false;
        }
        true
    }

    fn enabled(&self) -> bool {
        return self
            .screen
            .config
            .read()
            .unwrap()
            .config
            .bitpanda_screen_active;
    }

    fn set_status(&self, status: bool) {
        self.screen
            .config
            .write()
            .unwrap()
            .config
            .bitpanda_screen_active = status;
    }
}

impl BitpandaScreen {
    pub fn draw_wallet_value(&mut self, image: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, scale: Scale) {
        let wallet_value = format!("{: >10}€", self.wallet_value.lock().unwrap()).to_string();
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            0,
            0,
            scale,
            self.screen.font.as_ref().unwrap(),
            "Bitpanda",
        );
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            160,
            0,
            scale,
            self.screen.font.as_ref().unwrap(),
            &wallet_value,
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
            self.screen.font.as_ref().unwrap(),
            "Last update",
        );

        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            52,
            40,
            scale,
            self.screen.font.as_ref().unwrap(),
            &date_value.format("%d.%m.%Y %T").to_string(),
        );
    }

    fn update(&mut self) {
        let mut image = RgbImage::new(256, 64);
        let scale = Scale { x: 16.0, y: 16.0 };

        self.draw_wallet_value(&mut image, scale);
        self.draw_updated_at(&mut image, scale);
        self.screen.bytes = image.into_vec();
    }

    pub fn new(
        description: String,
        font: Option<Font<'static>>,
        config: Arc<RwLock<ConfigManager>>,
    ) -> Self {
        let this = BitpandaScreen {
            screen: Screen {
                description,
                font,
                config,
                ..Default::default()
            },
            wallet_value: Arc::new(Mutex::new(0.0)),
            last_update: Arc::new(Mutex::new(SystemTime::now())),
        };

        let builder = thread::Builder::new().name("JOB_EXECUTOR".into());

        *this.screen.handle.lock().unwrap() = Some(
            builder
                .spawn({
                    let this = this.clone();
                    move || loop {
                        while !this.screen.active.load(Ordering::Acquire) {
                            thread::park();
                        }
                        thread::sleep(Duration::from_millis(1000));
                        let value = this.wallet_value.lock().unwrap();
                        let mut elapsed = this.last_update.lock().unwrap();
                        match elapsed.elapsed() {
                            Ok(duration) => {
                                if (duration.as_secs() > 60
                                    || duration.as_secs() < 60 && *value == 0.0)
                                    && this
                                        .screen
                                        .config
                                        .read()
                                        .unwrap()
                                        .config
                                        .bitpanda_api_key
                                        .clone()
                                        != ""
                                {
                                    // unlock value mutex until request is done
                                    drop(value);
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
                                                        this.screen
                                                            .config
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
                                    *elapsed = SystemTime::now();
                                }
                            }
                            Err(e) => {
                                println!("Error: {:?}", e);
                            }
                        }

                        //io::Write::flush(&mut io::stdout()).expect("flush failed!");
                    }
                })
                .expect("Cannot create JOB_EXECUTOR thread"),
        );
        this
    }
}
