use crate::helpers::convert::to_wstring;
use crate::HIBERNATING;
use std::thread;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::um::libloaderapi::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

pub fn register_power_broadcast(
    wnd_proc: unsafe extern "system" fn(*mut HWND__, u32, usize, isize) -> isize,
) {
    thread::spawn(move || unsafe {
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as UINT,
            style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(std::ptr::null_mut()) as HINSTANCE,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: GetStockObject(WHITE_BRUSH as i32) as HBRUSH,
            lpszMenuName: std::ptr::null_mut(),
            lpszClassName: to_wstring("rust_window_class").as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        if RegisterClassExW(&wc) == 0 {
            panic!("RegisterClassEx failed");
        }

        let hwnd = CreateWindowExW(
            0,
            wc.lpszClassName,
            to_wstring("AwesomeEvents").as_ptr(),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            wc.hInstance,
            std::ptr::null_mut(),
        );
        if hwnd == std::ptr::null_mut() {
            panic!("CreateWindowEx failed");
        }

        ShowWindow(hwnd, SW_HIDE);

        let mut msg = MSG {
            hwnd: std::ptr::null_mut(),
            message: 0,
            wParam: 0,
            lParam: 0,
            time: 0,
            pt: POINT { x: 0, y: 0 },
        };
        loop {
            let res = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
            if res == 0 || res == -1 {
                break;
            }

            DispatchMessageW(&msg);
        }
    });
}
pub unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_POWERBROADCAST {
        *HIBERNATING.lock().unwrap() = wparam == PBT_APMSUSPEND;
    }

    if msg == WM_DESTROY {
        PostQuitMessage(0);
        return 0;
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}
