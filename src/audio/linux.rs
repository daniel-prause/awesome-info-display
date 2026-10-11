use reqwest::Url;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use zbus::blocking::Connection;
use zbus::blocking::Proxy;
use zbus::zvariant::OwnedValue;

use crate::helpers::master_volume::get_master_volume;
use crate::screens::media_info_screen::MusicPlayerInfo;

// Status mapping:
// 0 = Stopped, 1 = Playing, 3 = Paused
fn status_to_int(status: &str) -> isize {
    match status {
        "Playing" => 1,
        "Paused" => 3,
        _ => 0,
    }
}

fn metadata_string(metadata: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(|value| String::try_from(value.clone()).ok())
}

fn metadata_i64(metadata: &HashMap<String, OwnedValue>, key: &str) -> Option<i64> {
    metadata
        .get(key)
        .and_then(|value| i64::try_from(value.clone()).ok())
}

fn metadata_artists(metadata: &HashMap<String, OwnedValue>) -> String {
    metadata
        .get("xesam:artist")
        .and_then(|value| Vec::<String>::try_from(value.clone()).ok())
        .unwrap_or_default()
        .join(", ")
}

fn local_file_from_url(value: &str) -> Option<PathBuf> {
    let url = Url::parse(value).ok()?;
    if url.scheme() != "file" {
        return None;
    }
    url.to_file_path().ok()
}

fn read_cover(metadata: &HashMap<String, OwnedValue>) -> Vec<u8> {
    let Some(art_url) = metadata_string(metadata, "mpris:artUrl") else {
        return Vec::new();
    };

    // Read local artwork only. Remote HTTP URLs need a separate downloader.
    let Some(path) = local_file_from_url(&art_url) else {
        return Vec::new();
    };

    fs::read(path).unwrap_or_default()
}

pub fn get_music_player_info(info: &mut MusicPlayerInfo) -> Result<bool, Box<dyn Error>> {
    let connection = Connection::session()?;

    let dbus = zbus::blocking::fdo::DBusProxy::new(&connection)?;
    let names = dbus.list_names()?;

    let players: Vec<String> = names
        .into_iter()
        .map(|name| name.to_string())
        .filter(|name| name.starts_with("org.mpris.MediaPlayer2."))
        .collect();

    if players.is_empty() {
        *info = MusicPlayerInfo::default();
        return Ok(true);
    }
    let mut selected: Option<(String, String)> = None;

    // Prefer a player currently playing, then fall back to any available player.
    for name in &players {
        let proxy = Proxy::new(
            &connection,
            name.as_str(),
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        )?;

        let status: String = proxy
            .get_property("PlaybackStatus")
            .unwrap_or_else(|_| "Stopped".to_string());

        if status == "Playing" {
            selected = Some((name.clone(), status));
            break;
        }

        if selected.is_none() {
            selected = Some((name.clone(), status));
        }
    }

    let Some((name, status)) = selected else {
        let (volume, mute) = get_master_volume();
        info.system_volume = volume;
        info.mute = mute;

        return Ok(true);
    };

    let proxy = Proxy::new(
        &connection,
        name.as_str(),
        "/org/mpris/MediaPlayer2",
        "org.mpris.MediaPlayer2.Player",
    )?;

    let metadata: HashMap<String, OwnedValue> = proxy.get_property("Metadata").unwrap_or_default();
    let position: i64 = proxy.get_property("Position").unwrap_or(0);

    if players.is_empty() {
        info.player_active = false;
        return Ok(true);
    }

    let track_length =
        (metadata_i64(&metadata, "mpris:length").unwrap_or(0).max(0) / 1_000_000) as isize;

    let (volume, mute) = get_master_volume();
    info.system_volume = volume;
    info.mute = mute;
    info.playback_status = status_to_int(&status);

    if track_length == 0 {
        info.current_track_position = 0;
        return Ok(true);
    }

    info.player_active = true;

    info.track_length =
        (metadata_i64(&metadata, "mpris:length").unwrap_or(0).max(0) / 1_000_000) as isize;

    info.current_track_position = (position.max(0) / 1_000) as isize;

    info.title = metadata_string(&metadata, "xesam:title").unwrap_or_default();

    info.artist = metadata_artists(&metadata);

    info.filepath = metadata_string(&metadata, "xesam:url").unwrap_or_default();

    info.cover = read_cover(&metadata);

    Ok(true)
}
