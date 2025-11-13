use audiotags::Tag;
use image::EncodableLayout;
use winapi::ctypes::c_void;

use winsafe::{co, msg::WndMsg, HWND};
pub struct Cover {
    pub data: Vec<u8>,
}

pub fn extract_current_cover_path(window: &winsafe::HWND) -> String {
    let (_thread_id, process_id) = HWND::GetWindowThreadProcessId(&window);

    let winamp_process_handle =
        winsafe::HPROCESS::OpenProcess(co::PROCESS::VM_READ, false, process_id);
    match winamp_process_handle {
        Ok(winamp_process_handle) => {
            let psz_name = unsafe {
                window.SendMessage(WndMsg {
                    msg_id: co::WM::USER,
                    wparam: 0,
                    lparam: 3031,
                })
            };
            let mut buffer = vec![0u16; 2048];
            let buffer_bytes: &mut [u8] = unsafe {
                std::slice::from_raw_parts_mut(
                    buffer.as_mut_ptr() as *mut u8,
                    buffer.len() * 2, // u16 = 2 Bytes
                )
            };

            match winamp_process_handle.ReadProcessMemory(psz_name as *mut c_void, buffer_bytes) {
                Ok(_result) => {
                    let str_len = buffer.iter().position(|&x| x == 0).unwrap_or_default();
                    return String::from_utf16_lossy(&buffer[..str_len]);
                }
                Err(_) => {
                    return String::new();
                }
            }
        }
        Err(_) => return String::new(),
    }
}

pub fn extract_cover_image(path: &String) -> Option<Cover> {
    match Tag::new().read_from_path(path) {
        Ok(tag) => match tag.album_cover() {
            Some(cover) => {
                let cover_as_image = image::load_from_memory(cover.data as &[u8]);
                match cover_as_image {
                    Ok(cover_image) => {
                        match cover_image
                            .resize_exact(170, 170, image::imageops::FilterType::Lanczos3)
                            .as_mut_rgb8()
                        {
                            Some(resized_cover) => {
                                return Some(Cover {
                                    data: resized_cover.as_bytes().to_vec(),
                                });
                            }
                            None => {}
                        }
                    }
                    Err(_) => {}
                }
            }
            None => {}
        },
        Err(_) => {}
    }
    match alternative_cover(path) {
        Ok(cover) => return Some(cover),
        Err(_) => return None, // cover not found, we don't care for now, why
    }
}

pub fn extract_cover_path(original_path: &String) -> Option<String> {
    let patterns = [
        "Folder.jpg",
        "AlbumArtSmall.jpg",
        "AlbumArt.jpg",
        "Album.jpg",
        ".folder.png",
        "cover.jpg",
        "thumb.jpg",
        "*.jpg",
    ];

    for pattern in &patterns {
        // Glob-Muster erstellen
        let glob_pattern = format!("\\{}", pattern);
        match std::path::Path::new(original_path).parent() {
            Some(p) => match p.to_str() {
                Some(p) => {
                    let complete_path = format!("{}{}", p, glob_pattern);
                    if let Ok(entries) = glob::glob(&complete_path) {
                        for entry in entries {
                            if let Ok(path) = entry {
                                if path.is_file() {
                                    match path.into_os_string().into_string() {
                                        Ok(p) => return Some(p),
                                        Err(_) => {}
                                    }
                                }
                            }
                        }
                    }
                }
                None => {}
            },
            None => {}
        }
    }

    None
}

pub fn alternative_cover(original_path: &String) -> Result<Cover, std::io::Error> {
    // since NTFS does not differentiate between case sensitivity,
    // folder.jpg will be as appropriate as Folder.jpg.
    // TODO: replace me with a Dir.glob based approach.
    let folder_image = extract_cover_path(original_path);
    match folder_image {
        Some(path) => {
            let image = image::open(path.clone());
            match image {
                Ok(img) => {
                    return Result::Ok(Cover {
                        data: img
                            .resize_exact(170, 170, image::imageops::FilterType::Lanczos3)
                            .as_mut_rgb8()
                            .unwrap()
                            .to_vec(),
                    });
                }
                Err(e) => {
                    return Result::Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Could not open image! Reason: {:?}", e),
                    ));
                }
            }
        }
        None => {}
    }

    return Result::Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Cover not found",
    ));
}
