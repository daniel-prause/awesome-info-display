use crate::{
    config_manager::ConfigManager,
    screens::{BasicScreen, Screen, Screenable},
};
use chrono::{DateTime, Local};
use crossbeam_channel::{bounded, Receiver, Sender};
use error_chain::error_chain;
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};

use std::{
    rc::Rc,
    sync::{atomic::AtomicBool, atomic::Ordering, Arc, RwLock},
    thread,
    time::{Duration, SystemTime},
};

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

impl Screenable for BitpandaScreen {
    fn get_screen(&mut self) -> &mut Screen {
        &mut self.screen
    }
}

impl BasicScreen for BitpandaScreen {
    fn update(&mut self) {
        let wallet_info = self.receiver.try_recv();
        match wallet_info {
            Ok(wallet_info) => {
                self.draw_screen(wallet_info);
            }
            Err(_) => {}
        }
    }
}

impl BitpandaScreen {
    fn draw_screen(&mut self, wallet_info: WalletInfo) {
        // draw initial image
        let mut image = RgbImage::new(256, 64);
        let scale = Scale { x: 16.0, y: 16.0 };

        self.draw_wallet_value(wallet_info.wallet_value, &mut image, scale);
        self.draw_updated_at(wallet_info.last_update, &mut image, scale);
        self.screen.main_screen_bytes = image.into_vec();
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
            &self.screen.font,
            "Bitpanda",
        );
        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            112,
            0,
            scale,
            &self.screen.font,
            &format!("{: >16.2}€", wallet_value),
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
            &self.screen.font,
            "Last update",
        );

        draw_text_mut(
            image,
            Rgb([255u8, 255u8, 255u8]),
            52,
            40,
            scale,
            &self.screen.font,
            &date_value.format("%d.%m.%Y %T").to_string(),
        );
    }

    pub fn new(
        description: String,
        key: String,
        font: Rc<Font<'static>>,
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
                    let mut last_update = SystemTime::UNIX_EPOCH;
                    let sender = tx.to_owned();
                    let active = active.clone();
                    loop {
                        while !active.load(Ordering::Acquire) {
                            thread::park();
                        }
                        let bitpanda_api_key = config_manager
                            .read()
                            .unwrap()
                            .config
                            .bitpanda_api_key
                            .clone();

                        if !bitpanda_api_key.is_empty() {
                            match is_overdue(last_update) {
                                Ok(overdue) => {
                                    if overdue {
                                        last_update = SystemTime::now();
                                        match calculate_wallet(last_update, bitpanda_api_key) {
                                            Ok(wallet_info) => {
                                                sender.try_send(wallet_info).unwrap_or_default();
                                            }
                                            Err(e) => eprintln!("Error: {}", e),
                                        }
                                    }
                                }
                                Err(e) => eprintln!("Error: {}", e),
                            }
                        }

                        thread::sleep(Duration::from_millis(1000));
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

fn is_overdue(last_update: SystemTime) -> core::result::Result<bool, Box<dyn std::error::Error>> {
    let result = last_update.elapsed()?.as_secs() > 60;
    Ok(result)
}

fn calculate_wallet(
    last_update: SystemTime,
    bitpanda_api_key: String,
) -> core::result::Result<WalletInfo, Box<dyn std::error::Error>> {
    // 1. get current values for crypto coins
    let body = reqwest::blocking::get("https://api.bitpanda.com/v1/ticker");

    let client = reqwest::blocking::Client::new();
    let asset_values = body?.text();
    let wallet_values = client
        .get("https://api.bitpanda.com/v1/wallets")
        .header("X-API-KEY", bitpanda_api_key)
        .send();

    let wallet_json: Value = serde_json::from_str(wallet_values?.text()?.as_str())?;
    let wallets: Vec<Value> = serde_json::from_str(&wallet_json["data"].to_string())?;
    let assets: Value = serde_json::from_str(asset_values?.as_str())?;
    let mut sum = 0.0;
    for wallet in wallets {
        let asset_key = wallet["attributes"]["cryptocoin_symbol"]
            .as_str()
            .unwrap_or_default();
        if wallet["attributes"]["balance"] != "0.00000000" {
            let amount_of_eur = assets[asset_key]["EUR"]
                .as_str()
                .unwrap_or_default()
                .parse::<f64>()?;
            let amount_of_crypto = wallet["attributes"]["balance"]
                .as_str()
                .unwrap_or_default()
                .parse::<f64>()?;

            sum += amount_of_crypto * amount_of_eur;
        }
    }

    Ok(WalletInfo {
        wallet_value: ((sum * 100.0).round() / 100.0),
        last_update,
    })
}
