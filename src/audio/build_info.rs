#[cfg(target_os = "windows")]
use crate::screens::media_info_screen::MusicPlayerInfo;

#[cfg(target_os = "windows")]
pub fn build_music_player_info(music_player_info: &mut MusicPlayerInfo) {
    use winsafe::{HWND, co};

    fn deactivate(info: &mut MusicPlayerInfo) {
        info.player_active = false;
        info.playback_status = 0;
        info.current_track_position = 0;
        info.track_length = 0;
        info.filepath.clear();
    }

    let match_correct_artist_and_title_format = match regex::Regex::new(r"\s(.*)-") {
        Ok(regex) => regex,
        Err(err) => {
            eprintln!("REGEX ERROR: {err:?}");
            return;
        }
    };

    let match_artist_and_title = match regex::Regex::new(r"(.*?) - (.*)") {
        Ok(regex) => regex,
        Err(err) => {
            eprintln!("REGEX ERROR: {err:?}");
            return;
        }
    };

    let match_artist_or_title = match regex::Regex::new(r"\s(.*?)\s") {
        Ok(regex) => regex,
        Err(err) => {
            eprintln!("REGEX ERROR: {err:?}");
            return;
        }
    };

    let window = match HWND::FindWindow(Some(winsafe::AtomStr::from_str("Winamp v1.x")), None) {
        Ok(Some(window)) => window,
        _ => {
            deactivate(music_player_info);
            return;
        }
    };

    // Playback status: 1 = playing, 3 = paused.
    let playback_status = match send_winamp_message(&window, co::WM::USER, 0, 104) {
        Some(value) => value,
        None => {
            deactivate(music_player_info);
            return;
        }
    };

    music_player_info.playback_status = playback_status;

    let current_track_position = send_winamp_message(&window, co::WM::USER, 0, 105).unwrap_or(0);

    music_player_info.current_track_position = if playback_status == 1 || playback_status == 3 {
        current_track_position
    } else {
        0
    };

    // Track length in seconds.
    music_player_info.track_length =
        send_winamp_message(&window, co::WM::USER, 1, 105).unwrap_or(0);

    // Read the window caption without sending WM_GETTEXT manually.
    let data = match window.GetWindowText() {
        Ok(title) => title,
        Err(_) => {
            deactivate(music_player_info);
            return;
        }
    };

    music_player_info.player_active =
        !data.is_empty() && match_correct_artist_and_title_format.is_match(&data);

    let Some(caps) = match_correct_artist_and_title_format.captures(&data) else {
        deactivate(music_player_info);
        return;
    };

    let artist_and_title = caps.get(1).map_or("", |m| m.as_str()).trim();

    if let Some(caps) = match_artist_and_title.captures(artist_and_title) {
        music_player_info.artist = caps.get(1).map_or("", |m| m.as_str()).trim().to_string();

        music_player_info.title = caps.get(2).map_or("", |m| m.as_str()).trim().to_string();
    } else if let Some(caps) = match_artist_or_title.captures(&data) {
        music_player_info.title = caps.get(1).map_or("", |m| m.as_str()).trim().to_string();

        music_player_info.artist.clear();
    } else {
        deactivate(music_player_info);
    }
}

#[cfg(target_os = "windows")]
fn send_winamp_message(
    window: &winsafe::HWND,
    msg_id: winsafe::co::WM,
    wparam: usize,
    lparam: isize,
) -> Option<isize> {
    unsafe {
        window
            .SendMessageTimeout(
                winsafe::msg::Wm {
                    msg_id,
                    wparam,
                    lparam,
                },
                winsafe::co::SMTO::ABORTIFHUNG | winsafe::co::SMTO::ERRORONEXIT,
                100,
            )
            .ok()
    }
}

#[cfg(target_os = "linux")]
pub fn build_music_player_info(music_player_info: &mut MusicPlayerInfo) {
    use crate::audio::linux::get_music_player_info;

    // we don't care; if the music info is empty, its empty - regardless of the reason.
    get_music_player_info(music_player_info).unwrap_or_default();
}
