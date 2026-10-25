#![no_std]
#![no_main]
#![windows_subsystem = "windows"]
//extern crate alloc;

use core::panic::PanicInfo;
use core::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::ExitProcess;
use windows_sys::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DefWindowProcW, PostQuitMessage, RegisterClassExW, ShowWindow, SW_SHOWNORMAL, MSG, PM_REMOVE, WM_DESTROY, WM_QUIT, WNDCLASSEXW, WS_OVERLAPPEDWINDOW, DispatchMessageW, TranslateMessage, PeekMessageW};
use windows_sys::w;
use windows_sys::Win32::Graphics::Gdi::{GetDC, ReleaseDC, BITMAPINFO, CreateBitmap, StretchDIBits};

#[unsafe(no_mangle)]
pub extern "system" fn mainCRTStartup() -> ! {
    let window_class_name = w!("my window class");


    let instance = unsafe {
        GetModuleHandleW(core::ptr::null())
    };

    let window_class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: null_mut(),
        hCursor: null_mut(),
        hbrBackground: null_mut(),
        lpszMenuName: null(),
        lpszClassName: window_class_name,
        hIconSm: null_mut(),
    };

    let atom = unsafe {
        RegisterClassExW(&window_class)
    };

    if atom == 0 {
        unsafe {
            ExitProcess(1);
        }
    }

    let window;
    unsafe {
        window = CreateWindowExW(
            0,
            window_class_name,
            w!("Rust no_std Renderer"),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            1280,
            720,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );

        if window.is_null() {
            ExitProcess(2);
        }


        ShowWindow(window, SW_SHOWNORMAL);


        let mut msg: MSG = unsafe {
            core::mem::zeroed()
        };

        loop {
            while unsafe {
                PeekMessageW(
                    &mut msg,
                    null_mut(),
                    0,
                    0,
                    PM_REMOVE,
                )
            } != 0
            {
                if msg.message == WM_QUIT {
                    unsafe {
                        ExitProcess(msg.wParam as u32);
                    }
                }

                unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            // work flow


        }
    }
}
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        },

        _ => {
            DefWindowProcW(
                hwnd,
                msg,
                wparam,
                lparam,
            )
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        ExitProcess(1);
    }
}
#[link(name = "ucrt")]
#[link(name = "vcruntime")]
unsafe extern "C" {}

