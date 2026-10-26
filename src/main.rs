#![no_std]
#![no_main]
#![windows_subsystem = "windows"]
//extern crate alloc;

mod math;

use crate::math::Vec4;
use core::panic::PanicInfo;
use core::ptr::{null, null_mut};
use core::slice;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{GetDC, ReleaseDC, StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, COLORONCOLOR, DIB_RGB_COLORS, RGBQUAD, SRCCOPY, SetStretchBltMode, SetBrushOrgEx, HALFTONE};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{GetProcessHeap, HEAP_ZERO_MEMORY, HeapAlloc};
use windows_sys::Win32::System::SystemInformation::GetTickCount64;
use windows_sys::Win32::System::Threading::{ExitProcess, Sleep};
use windows_sys::Win32::UI::WindowsAndMessaging::{AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, PeekMessageW, PostQuitMessage, RegisterClassExW, ShowWindow, TranslateMessage, MSG, PM_REMOVE, SW_SHOWNORMAL, WM_DESTROY, WM_QUIT, WNDCLASSEXW, WS_OVERLAPPEDWINDOW};
use windows_sys::w;

const FRAME_HEIGHT: i32 = 1080;//9 * 10;
const FRAME_WIDTH:i32= 1920;//16*10;

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
#[derive(Clone, Copy)]
struct VertexModel{
    x_model:f32,
    y_model:f32,
    z_model:f32,

    r:u8,
    g:u8,
    b:u8,
}

#[derive(Clone, Copy)]
struct VertexClip{
    x:f32,
    y:f32,
    z:f32,
    w:f32,

    r:f32,
    g:f32,
    b:f32,
}

#[derive(Clone, Copy)]
struct VertexProcessed{
    processed: bool,

    screen_x:f32,
    screen_y:f32,

    inv_w: f32,

    r_over_w:f32,
    g_over_w:f32,
    b_over_w:f32
}

struct Model<'a> {
    vertices:&'a [VertexModel],
    vertex_clips:&'a mut [VertexClip],
    vertex_processed:&'a mut [VertexProcessed],

    indices:&'a [u32],
}

struct Camera {
    matrix :math::Mat4,

}



#[unsafe(no_mangle)]
pub extern "system" fn mainCRTStartup() -> ! {
    let mut vertices : [VertexModel;3] = [
        VertexModel{
            x_model: 1.0,
            y_model:-0.5,
            z_model: 2.0,
            r: 255,
            g: 0,
            b: 0,
        },
        VertexModel{
            x_model: -1.0,
            y_model: -0.5,
            z_model: 2.0,
            r: 0,
            g: 255,
            b: 0,
        },
        VertexModel{
            x_model: 0.0,
            y_model: 0.5,
            z_model: 2.0,
            r: 0,
            g: 0,
            b: 255,
        }
    ];

    let indices :[u32;3]= [0,1,2];

    let model;

    let camera;



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
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 1280,
            bottom: 720,
        };

        AdjustWindowRectEx(
            &mut rect,
            WS_OVERLAPPEDWINDOW,
            0,
            0,
        );


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


        //test----
        let vertex_clips_len = vertices.len();
        let vertex_clips_bytes = vertex_clips_len*size_of::<VertexClip>();

        let vertex_processed_len =  vertices.len();
        let vertex_processed_bytes = vertex_processed_len*size_of::<VertexProcessed>();

        let sum_byte = vertex_processed_bytes + vertex_clips_bytes;
        let model_heap = HeapAlloc(process_heap, HEAP_ZERO_MEMORY, sum_byte);

        if model_heap.is_null() {
            ExitProcess(1);
        }

        let vertex_clips_ptr = model_heap as *mut VertexClip;
        let vertex_processed_ptr = (model_heap as *mut u8).add(vertex_clips_bytes) as *mut VertexProcessed;

        let mut vertex_clips = slice::from_raw_parts_mut(vertex_clips_ptr, vertex_clips_len);
        let mut vertex_processed = slice::from_raw_parts_mut(vertex_processed_ptr, vertex_processed_len);
        model = Model{
            vertices: &mut vertices,
            vertex_clips: &mut vertex_clips,
            vertex_processed:&mut vertex_processed,
            indices: & indices,
        };


        let matrix = math::Mat4::perspective(90 as f32*0.017453292519943295, FRAME_WIDTH as f32 / FRAME_HEIGHT as f32,0.1,100.0);
        camera = Camera{
            matrix:matrix,
        };
        //----


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

            for i in 0..model.vertex_processed.len() {
                model.vertex_processed[i].processed = false;
            }

            for i in 0..model.vertices.len(){
                let vec = Vec4{
                    x:model.vertices[i].x_model,
                    y:model.vertices[i].y_model,
                    z:model.vertices[i].z_model,
                    w:1.0
                };
                let clip = math::Mat4::mul_vec4(&camera.matrix, vec);

                model.vertex_clips[i].x = clip.x;
                model.vertex_clips[i].y = clip.y;
                model.vertex_clips[i].z = clip.z;
                model.vertex_clips[i].w = clip.w;
                model.vertex_clips[i].r = model.vertices[i].r as f32 /255.0;
                model.vertex_clips[i].g = model.vertices[i].g as f32 /255.0;
                model.vertex_clips[i].b = model.vertices[i].b as f32 /255.0;
            }

            for i in 0..indices.len()/3{
                let idx0 = indices[i*3] as usize;
                let idx1 = indices[i*3+1] as usize;
                let idx2 = indices[i*3+2] as usize;

                if model.vertex_clips[idx0].z < 0.1 || model.vertex_clips[idx1].z < 0.1 || model.vertex_clips[idx2].z < 0.1 {
                    continue;
                };

                if !model.vertex_processed[idx0].processed{
                    process_vertex(&model.vertex_clips[idx0], &mut model.vertex_processed[idx0])
                }
                if !model.vertex_processed[idx1].processed{
                    process_vertex(&model.vertex_clips[idx1], &mut model.vertex_processed[idx1])
                }
                if !model.vertex_processed[idx2].processed{
                    process_vertex(&model.vertex_clips[idx2], &mut model.vertex_processed[idx2])
                }
                rasterize(
                    & model.vertex_processed[idx0],
                    & model.vertex_processed[idx1],
                    & model.vertex_processed[idx2],
                    framebuffer
                );

            }

            present_frame(window,&bitmap_info,framebuffer, FRAME_WIDTH, FRAME_HEIGHT);

        }
    }
}

fn rasterize(vertex_idx0:&VertexProcessed,vertex_idx1:&VertexProcessed,vertex_idx2:&VertexProcessed, framebuffer:&mut [Pixel]) {
    let area = edge(
        vertex_idx0.screen_x,
        vertex_idx0.screen_y,
        vertex_idx1.screen_x,
        vertex_idx1.screen_y,
        vertex_idx2.screen_x,
        vertex_idx2.screen_y,
    );

    if area > -0.00001 && area < 0.00001 {
        return;
    }
    /*let areaDenom = (vertex_idx1.screen_y-vertex_idx2.screen_y) * (vertex_idx0.screen_x-vertex_idx2.screen_x)+ (vertex_idx2.screen_x-vertex_idx1.screen_x) * (vertex_idx0.screen_y-vertex_idx2.screen_y);

    if areaDenom < 0.00001 {
        return;
    */
    let min_x = unsafe {
        floorf(
            vertex_idx0.screen_x
                .min(vertex_idx1.screen_x)
                .min(vertex_idx2.screen_x)
                .max(0.0)
        ) as i32
    };

    let max_x = unsafe {
        ceilf(
            vertex_idx0.screen_x
                .max(vertex_idx1.screen_x)
                .max(vertex_idx2.screen_x)
                .min((FRAME_WIDTH - 1) as f32)
        ) as i32
    };

    let min_y = unsafe {
        floorf(
            vertex_idx0.screen_y
                .min(vertex_idx1.screen_y)
                .min(vertex_idx2.screen_y)
                .max(0.0)
        ) as i32
    };

    let max_y = unsafe {
        ceilf(
            vertex_idx0.screen_y
                .max(vertex_idx1.screen_y)
                .max(vertex_idx2.screen_y)
                .min((FRAME_HEIGHT - 1) as f32)
        ) as i32
    };



    if min_x>max_x || min_y>max_y {
        return;
    }

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let px_half = px as f32 + 0.5;
            let py_half = py as f32 + 0.5;


            /*
            let w0 = ((vertex_idx1.screen_y - vertex_idx2.screen_y)*(px_half-vertex_idx2.screen_x)+(vertex_idx2.screen_x-vertex_idx1.screen_x)*(py_half-vertex_idx2.screen_y))/areaDenom;
            let w1 = ((vertex_idx2.screen_y - vertex_idx0.screen_y)*(px_half-vertex_idx2.screen_x)+(vertex_idx0.screen_x-vertex_idx2.screen_x)*(py_half-vertex_idx2.screen_y))/areaDenom;
            let w2 = 1.0 - w0 - w1;

            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0{
                continue;
            }
*/
            let e0 = edge(
                vertex_idx1.screen_x,
                vertex_idx1.screen_y,
                vertex_idx2.screen_x,
                vertex_idx2.screen_y,
                px_half,
                py_half,
            );

            let e1 = edge(
                vertex_idx2.screen_x,
                vertex_idx2.screen_y,
                vertex_idx0.screen_x,
                vertex_idx0.screen_y,
                px_half,
                py_half,
            );

            let e2 = edge(
                vertex_idx0.screen_x,
                vertex_idx0.screen_y,
                vertex_idx1.screen_x,
                vertex_idx1.screen_y,
                px_half,
                py_half,
            );
            if area > 0.0 {
                if e0 < 0.0 || e1 < 0.0 || e2 < 0.0 {
                    continue;
                }
            } else {
                if e0 > 0.0 || e1 > 0.0 || e2 > 0.0 {
                    continue;
                }
            }
            let inv_area = 1.0 / area;
            let w0 = e0 * inv_area;
            let w1 = e1 * inv_area;
            let w2 = e2 * inv_area;


            let pix_index = py * FRAME_WIDTH + px;

            let r_over_w = w0 * vertex_idx0.r_over_w + w1 *vertex_idx1.r_over_w+ w2 *vertex_idx2.r_over_w;
            let g_over_w = w0 * vertex_idx0.g_over_w + w1 *vertex_idx1.g_over_w+ w2 *vertex_idx2.g_over_w;
            let b_over_w = w0 * vertex_idx0.b_over_w + w1 *vertex_idx1.b_over_w+ w2 *vertex_idx2.b_over_w;

            let inv_w =
                w0 * vertex_idx0.inv_w
                    + w1 * vertex_idx1.inv_w
                    + w2 * vertex_idx2.inv_w;

            let r = r_over_w / inv_w;
            let g = g_over_w / inv_w;
            let b = b_over_w / inv_w;

            let mut pixel = framebuffer[pix_index as usize];

            pixel.r =( r*255.0 ) as u8;
            pixel.g =( g*255.0 ) as u8;
            pixel.b =( b*255.0 ) as u8;

            framebuffer[pix_index as usize] = pixel;
        }
    }
}

fn edge(
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    px: f32,
    py: f32,
) -> f32 {
    (bx - ax) * (py - ay) - (by - ay) * (px - ax)
}
fn process_vertex(vertex_clip: &VertexClip, vertex_processed:&mut VertexProcessed){
    vertex_processed.processed = true;

    vertex_processed.screen_x = (vertex_clip.x / vertex_clip.w + 1.0) * 0.5 * FRAME_WIDTH as f32;// чи тут потрібні розміри client space?
    vertex_processed.screen_y = (1.0 - vertex_clip.y / vertex_clip.w) * 0.5 * FRAME_HEIGHT as f32;

    vertex_processed.inv_w = 1.0 / vertex_clip.w;

    vertex_processed.r_over_w = vertex_clip.r * vertex_processed.inv_w;
    vertex_processed.g_over_w = vertex_clip.g * vertex_processed.inv_w;
    vertex_processed.b_over_w = vertex_clip.b * vertex_processed.inv_w;
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

        SetStretchBltMode(
            hdc,
            COLORONCOLOR,
        );

        //SetStretchBltMode(hdc, HALFTONE);
        //SetBrushOrgEx(hdc, 0, 0, null_mut());

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
unsafe extern "C" {
    fn tanf(x: f32) -> f32;
    fn floorf(x: f32) -> f32;
    fn ceilf(x: f32) -> f32;
}

