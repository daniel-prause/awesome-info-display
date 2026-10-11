use crate::screens::media_info_screen::MusicPlayerInfo;

#[cfg(target_os = "windows")]
pub fn build_music_player_info() -> MusicPlayerInfo {
    let mut music_player_info: MusicPlayerInfo = Default::default();

    match winsafe::HWND::FindWindow(Some(winsafe::AtomStr::from_str("Winamp v1.x")), None) {
        Ok(window) => {
            match window {
                Some(window) => {
                    // 1 == playing, 3 == paused, anything else == stopped
                    //let playback_status = SendMessageW(hwnd, WM_USER, 0, 104);
                    let playback_status = unsafe {
                        window.SendMessage(msg::Wm {
                            msg_id: co::WM::USER,
                            wparam: 0,
                            lparam: 104,
                        })
                    };
                    music_player_info.playback_status = playback_status;
                    // current position in msecs
                    let mut current_track_position = unsafe {
                        window.SendMessage(msg::Wm {
                            msg_id: co::WM::USER,
                            wparam: 0,
                            lparam: 105,
                        })
                    };
                    if playback_status != 1 && playback_status != 3 {
                        current_track_position = 0;
                    }

                    music_player_info.current_track_position = current_track_position;

                    // track length in seconds (multiply by thousand)
                    let track_length = unsafe {
                        window.SendMessage(msg::Wm {
                            msg_id: co::WM::USER,
                            wparam: 1,
                            lparam: 105,
                        })
                    };
                    music_player_info.track_length = track_length;
                    // get title
                    let current_index = unsafe {
                        window.SendMessage(msg::Wm {
                            msg_id: co::WM::USER,
                            wparam: 0,
                            lparam: 125,
                        })
                    };
                    let title_length = unsafe {
                        window.SendMessage(msg::Wm {
                            msg_id: co::WM::GETTEXTLENGTH,
                            wparam: current_index as usize,
                            lparam: 0,
                        })
                    };

                    let path = extract_current_cover_path(&window);

                    if path != cover_manager.last_path {
                        let file_exists = Path::new(std::ffi::OsStr::new(&path)).exists();
                        if file_exists {
                            music_player_info.filepath = path.clone();
                            match extract_cover_image(&path) {
                                Some(cover) => {
                                    music_player_info.cover = cover.data.as_bytes().to_vec();
                                    cover_manager.last_path = path.clone();
                                    cover_manager.current_cover = cover.data.as_bytes().to_vec();
                                }
                                None => {}
                            }
                        } else {
                            music_player_info.cover = vec![211; 320 * 170 * 3];
                        }
                    } else {
                        music_player_info.cover = cover_manager.current_cover.clone();
                    }

                    let buffer_length = title_length + 1;
                    let mut buffer = Vec::<u16>::with_capacity(buffer_length as usize);
                    buffer.resize(buffer_length as usize, 0);

                    unsafe {
                        window.SendMessage(msg::Wm {
                            msg_id: co::WM::GETTEXT,
                            wparam: buffer_length as usize,
                            lparam: buffer.as_mut_ptr() as LPARAM,
                        })
                    };

                    let data = String::from_utf16_lossy(&buffer);

                    music_player_info.player_active =
                        title_length > 0 && match_correct_artist_and_title_format.is_match(&data);

                    let caps = match_correct_artist_and_title_format.captures(&data);
                    match caps {
                        Some(caps) => {
                            let artist_and_title = caps.get(1).map_or("", |m| m.as_str()).trim();

                            let artist_and_title_caps =
                                match_artist_and_title.captures(artist_and_title);
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
                                            music_player_info.artist = "".into();
                                        }
                                        None => {
                                            music_player_info.player_active = false;
                                            music_player_info.filepath.clear();
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            music_player_info.player_active = false;
                            music_player_info.filepath.clear();
                        }
                    }
                }
                None => {
                    music_player_info.player_active = false;
                    music_player_info.filepath.clear();
                }
            }
        }
        Err(_) => {
            music_player_info.player_active = false;
            music_player_info.filepath.clear();
        }
    }
}

#[cfg(target_os = "linux")]
pub fn build_music_player_info(music_player_info: &mut MusicPlayerInfo) {
    use crate::audio::linux::get_music_player_info;

    // we don't care; if the music info is empty, its empty - regardless of the reason.
    get_music_player_info(music_player_info).unwrap_or_default();
}
