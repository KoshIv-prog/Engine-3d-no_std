#![no_std]
#![no_main]
#![windows_subsystem = "windows"]
//extern crate alloc;

use core::panic::PanicInfo;
use core::ptr::{null, null_mut};
use core::slice;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, GetDC, RGBQUAD, ReleaseDC, SRCCOPY, StretchDIBits};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{GetProcessHeap, HEAP_ZERO_MEMORY, HeapAlloc};
use windows_sys::Win32::System::SystemInformation::GetTickCount64;
use windows_sys::Win32::System::Threading::{ExitProcess, Sleep};
use windows_sys::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, MSG, PM_REMOVE, PeekMessageW, PostQuitMessage, RegisterClassExW, SW_SHOWNORMAL, ShowWindow, TranslateMessage, WM_DESTROY, WM_QUIT, WNDCLASSEXW, WS_OVERLAPPEDWINDOW};
use windows_sys::w;

const FRAME_HEIGHT: i32 = 9 * 10;
const FRAME_WIDTH:i32= 16*10;

#[repr(C)]
#[derive(Clone, Copy)]
struct Pixel {
    b: u8,
    g: u8,
    r: u8,
    _reserved: u8,
}

impl Pixel{
    const fn rgb(r:u8,g:u8,b:u8) -> Self{
        Pixel{
            r:r,
            g:g,
            b:b,
            _reserved:0,
        }
    }

}

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

        let process_heap = GetProcessHeap();
        let framebuffer_len =
            match FRAME_WIDTH.checked_mul(FRAME_HEIGHT) {
                Some(x) => x as usize,
                None => ExitProcess(1),
            };
        let framebuffer_bytes =
            match framebuffer_len.checked_mul(size_of::<Pixel>()){
                Some(x) => x,
                None => ExitProcess(1),
            };


        let heap = HeapAlloc(process_heap, HEAP_ZERO_MEMORY, framebuffer_bytes);
        if heap.is_null() {
            ExitProcess(1);
        }
        let frame_buffer_ptr = heap as *mut Pixel;
        let framebuffer = slice::from_raw_parts_mut(frame_buffer_ptr, framebuffer_len);



        let bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,

                biWidth: FRAME_WIDTH,

                biHeight: -FRAME_HEIGHT,

                biPlanes: 1,
                biBitCount: 32,

                biCompression: BI_RGB,

                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },

            bmiColors: [RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }],
        };

        ShowWindow(window, SW_SHOWNORMAL);


        let mut msg: MSG = unsafe {
            core::mem::zeroed()
        };

        let mut last_time = GetTickCount64();
        let mut x = 0;
        let mut y = 0;

        let mut vel_x = 1;
        let mut vel_y = 1;

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

            if GetTickCount64() - last_time < 100 {
                Sleep(10);
                continue;
            }

            last_time = GetTickCount64();

            // work flow
            framebuffer.fill(Pixel::rgb(255,255,255));

            if x + vel_x >= FRAME_WIDTH || x + vel_x < 0 {
                vel_x = -vel_x;
            }
            if y + vel_y >= FRAME_HEIGHT || y + vel_y < 0{
                vel_y = -vel_y;
            }
            x = x + vel_x;
            y = y + vel_y;

            framebuffer[(y*FRAME_WIDTH + x) as usize] = Pixel::rgb(0,0,0);


            present_frame(window,&bitmap_info,framebuffer, FRAME_WIDTH, FRAME_HEIGHT);

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

fn present_frame(
    window: HWND,
    bitmap_info: &BITMAPINFO,
    framebuffer: & [Pixel],
    frame_width: i32,
    frame_height: i32,
) {
    unsafe {
        let mut rect = core::mem::zeroed();

        if GetClientRect(window, &mut rect) == 0 {
            return;
        }

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        if width <= 0 || height <= 0 {
            return;
        }

        let hdc = GetDC(window);

        if hdc.is_null() {
            return;
        }

        let lines = StretchDIBits(
            hdc,

            0,
            0,
            width,
            height,

            0,
            0,
            frame_width,
            frame_height,

            framebuffer.as_ptr() as  _,

            bitmap_info,
            DIB_RGB_COLORS,
            SRCCOPY
        );

        ReleaseDC(window, hdc);


        if lines == 0 {
            // presentation failed
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

