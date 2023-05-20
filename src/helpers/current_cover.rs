use std::ptr::null_mut;

use audiotags::Tag;
use image::EncodableLayout;
use winapi::{
    ctypes::c_void,
    shared::minwindef::DWORD,
    um::{
        winnt::HANDLE,
        winuser::{FindWindowW, SendMessageW, WM_USER},
    },
};

use crate::helpers::convert::to_wstring;

pub struct Cover {
    pub data: Vec<u8>,
    pub filepath: String,
}

pub fn extract_current_cover_path(mut winamp_process_handle: HANDLE) -> String {
    unsafe {
        let winname: Vec<u16> = to_wstring("Winamp v1.x");
        // wide string
        let hwnd: winapi::shared::windef::HWND = FindWindowW(winname.as_ptr(), null_mut());

        let mut process_id: DWORD = 0;
        winapi::um::winuser::GetWindowThreadProcessId(hwnd, &mut process_id);

        if winamp_process_handle == null_mut() {
            winamp_process_handle =
                winapi::um::processthreadsapi::OpenProcess(0x0010, 0, process_id);
        }

        let psz_name = SendMessageW(hwnd, WM_USER, 0, 3031);
        let mut buffer = Vec::<u16>::with_capacity(2048_usize);
        buffer.resize(2048_usize, 0);

        let mut number_read = 0;

        winapi::um::memoryapi::ReadProcessMemory(
            winamp_process_handle,
            psz_name as *const c_void,
            buffer.as_mut_ptr().cast(),
            2048,
            &mut number_read,
        );

        /* str via strlen */
        let str_len = buffer.iter().position(|x| *x == 0).unwrap_or_default();
        String::from_utf16_lossy(&buffer[0..str_len])
    }
}

pub fn extract_cover_image(path: &String) -> Option<Cover> {
    match Tag::new().read_from_path(path) {
        Ok(tag) => {
            match tag.album_cover() {
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
                                        filepath: path.clone(),
                                    });
                                }
                                None => {}
                            }
                        }
                        Err(_) => {}
                    }
                }
                None => {
                    // do nothing - just like self help singh!
                }
            }
        }
        Err(_) => {}
    }
    None
}
