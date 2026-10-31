#![no_std]
#![no_main]
#![windows_subsystem = "windows"]
//extern crate alloc;

mod math;
mod rasrerizator;
mod clipping;
mod loader;
mod objreader;
mod mesh;
mod renderer_workspace;

use crate::clipping::near_plane_clip;
use crate::math::{Mat4, Vec4};
use crate::mesh::Mesh;
use crate::objreader::{parse_obj};
use crate::rasrerizator::rasterize;
use core::panic::PanicInfo;
use core::ptr::{null, null_mut};
use core::slice;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{BITMAPINFO, BITMAPINFOHEADER, BI_RGB, COLORONCOLOR, DIB_RGB_COLORS, GetDC, RGBQUAD, ReleaseDC, SRCCOPY, SetStretchBltMode, StretchDIBits};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{GetProcessHeap, HEAP_ZERO_MEMORY, HeapAlloc};
use windows_sys::Win32::System::SystemInformation::GetTickCount64;
use windows_sys::Win32::System::Threading::{ExitProcess, Sleep};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_A, VK_D, VK_S, VK_SHIFT, VK_SPACE, VK_W};
use windows_sys::Win32::UI::WindowsAndMessaging::{AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, MSG, PM_REMOVE, PeekMessageW, PostQuitMessage, RegisterClassExW, SW_SHOWNORMAL, ShowWindow, TranslateMessage, WM_DESTROY, WM_QUIT, WNDCLASSEXW, WS_OVERLAPPEDWINDOW};
use windows_sys::core::PCWSTR;
use windows_sys::w;

const FRAME_HEIGHT: i32 = 720; //9 * 10;
const FRAME_WIDTH:i32= 1280;//16*10;

const SCREEN_HEIGHT :i32 = 720;
const SCREEN_WIDTH :i32 = 1280;
const TO_RADIANS: f32 = 0.017453292519943295;

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

struct Mesh_depr<'a> {
    vertices:&'a [VertexModel],
    vertex_clips:&'a mut [VertexClip],
    vertex_processed:&'a mut [VertexProcessed],

    indices:&'a [u32],
}
struct Object {
    mesh_id: usize,
    transform: Transform,
}

impl Object {
    fn rebuild_transform(&mut self){
        let mut transform = &mut self.transform;
        let scale = Mat4::scale(
            transform.scale_x,
            transform.scale_y,
            transform.scale_z,
        );

        let rx = Mat4::rotation_x(transform.rotation_x);
        let ry = Mat4::rotation_y(transform.rotation_y);
        let rz = Mat4::rotation_z(transform.rotation_z);

        let translation = Mat4::translation(
            transform.x,
            transform.y,
            transform.z,
        );

        let rxy = Mat4::mul(&ry, &rx);
        let rotation = Mat4::mul(&rz, &rxy);
        let scaled = Mat4::mul(&rotation, &scale);

        transform.matrix = Mat4::mul(&translation, &scaled);
        transform.dirty = false;
    }
}
struct Camera {
    x: f32,
    y: f32,
    z: f32,

    yaw: f32,
    pitch: f32,
    roll: f32,

    fov_x: f32,
    near: f32,
    far: f32,

    view: Mat4,
    projection: Mat4,
    view_projection: Mat4,

    view_dirty: bool,
    projection_dirty: bool,
}
impl Camera {
    fn new(
        fov_x: f32,
        near: f32,
        far: f32,
    ) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,

            pitch: 0.0,
            yaw: 0.0,
            roll: 0.0,

            fov_x,
            near,
            far,

            view: Mat4::identity(),
            projection: Mat4::identity(),
            view_projection: Mat4::identity(),

            view_dirty: true,
            projection_dirty: true,
        }
    }
    fn rebuild_view(&mut self) {
        let translation = Mat4::translation(
            -self.x,
            -self.y,
            -self.z,
        );

        let rx = Mat4::rotation_x(-self.pitch);
        let ry = Mat4::rotation_y(-self.yaw);
        let rz = Mat4::rotation_z(-self.roll);

        let temp0 = Mat4::mul(&rz, &translation);
        let temp1 = Mat4::mul(&ry, &temp0);

        self.view = Mat4::mul(&rx, &temp1);

        self.view_dirty = false;
    }
    fn rebuild_projection(
        &mut self,
        aspect: f32,
    ) {
        self.projection = Mat4::perspective(
            self.fov_x,
            aspect,
            self.near,
            self.far,
        );

        self.projection_dirty = false;
    }
    fn rebuild_view_projection(&mut self) {
        self.view_projection =
            Mat4::mul(
                &self.projection,
                &self.view,
            );
    }
}

struct Transform {
    x: f32,
    y: f32,
    z: f32,

    rotation_x: f32,
    rotation_y: f32,
    rotation_z: f32,

    scale_x: f32,
    scale_y: f32,
    scale_z: f32,

    matrix: Mat4,
    dirty: bool,
}


#[unsafe(no_mangle)]
pub extern "system" fn mainCRTStartup() -> ! {
    let mut vertices: [VertexModel; 8] = [
        VertexModel {
            x_model: -0.5,
            y_model: 1.0,
            z_model: 0.5,
            r: 255,
            g: 0,
            b: 0,
        },
        VertexModel {
            x_model: 0.5,
            y_model: 1.0,
            z_model: 0.5,
            r: 0,
            g: 255,
            b: 0,
        },
        VertexModel {
            x_model: 0.5,
            y_model: 1.0,
            z_model: -0.5,
            r: 0,
            g: 0,
            b: 255,
        },
        VertexModel {
            x_model: -0.5,
            y_model: 1.0,
            z_model: -0.5,
            r: 255,
            g: 255,
            b: 0,
        },
        VertexModel {
            x_model: -0.5,
            y_model: 0.0,
            z_model: 0.5,
            r: 255,
            g: 0,
            b: 255,
        },
        VertexModel {
            x_model: 0.5,
            y_model: 0.0,
            z_model: 0.5,
            r: 0,
            g: 255,
            b: 255,
        },
        VertexModel {
            x_model: 0.5,
            y_model: 0.0,
            z_model: -0.5,
            r: 255,
            g: 255,
            b: 255,
        },
        VertexModel {
            x_model: -0.5,
            y_model: 0.0,
            z_model: -0.5,
            r: 255,
            g: 128,
            b: 128,
        }
    ];

    let indices: [u32; 36] = [
        0, 1, 2,
        0, 2, 3,

        4, 5, 6,
        4, 6, 7,

        0, 1, 4,
        1, 4, 5,

        2, 3, 6,
        3, 6, 7,

        1, 2, 5,
        2, 5, 6,

        0, 3, 4,
        3, 4, 7
    ];



    let mut temp_registry:[Mesh_depr;1];

    let mut camera;
    let mut objects;

    let window_class_name = w!("my window class");

    let instance = unsafe {
        GetModuleHandleW(null())
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

    unsafe {
        let atom = RegisterClassExW(&window_class);
        if atom == 0 {
            ExitProcess(1)
        }
    }


    let mut rect = RECT {
        left: 0,
        top: 0,
        right: SCREEN_WIDTH,
        bottom: SCREEN_HEIGHT,
    };
    unsafe {
        AdjustWindowRectEx(
            &mut rect,
            WS_OVERLAPPEDWINDOW,
            0,
            0,
        );
    }

    let window_width = rect.right - rect.left;
    let window_height = rect.bottom - rect.top;

    let window;
    unsafe {
        window = CreateWindowExW(
            0,
            window_class_name,
            w!("Rust no_std Renderer"),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            window_width,
            window_height,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );
        if window.is_null() {
            ExitProcess(1)
        }
    }


    let process_heap = unsafe { GetProcessHeap() };

    let framebuffer_ptr;
    let z_buffer_ptr;
    let framebuffer;
    let z_buffer;
    let screen_heap;
    unsafe {
        let framebuffer_len =
            match FRAME_WIDTH.checked_mul(FRAME_HEIGHT) {
                Some(x) => x as usize,
                None => ExitProcess(1),
            };
        let framebuffer_bytes =
            match framebuffer_len.checked_mul(size_of::<Pixel>()) {
                Some(x) => x,
                None => ExitProcess(1),
            };

        let z_buffer_len =
            match FRAME_HEIGHT.checked_mul(FRAME_WIDTH) {
                Some(x) => x as usize,
                None => ExitProcess(1),
            };
        let z_buffer_bytes =
            match z_buffer_len.checked_mul(size_of::<f32>()) {
                Some(x) => x,
                None => ExitProcess(1),
            };

        let target_screen_buffer_size =
            match framebuffer_bytes.checked_add(z_buffer_bytes) {
                Some(x) => x,
                None => ExitProcess(1),
            };

        screen_heap = HeapAlloc(process_heap, HEAP_ZERO_MEMORY, target_screen_buffer_size);
        if screen_heap.is_null() {
            ExitProcess(1);
        }
        framebuffer_ptr = screen_heap as *mut Pixel;
        z_buffer_ptr = (screen_heap as *mut u8).add(framebuffer_bytes) as *mut f32;

        framebuffer = slice::from_raw_parts_mut(framebuffer_ptr, framebuffer_len);
        z_buffer = slice::from_raw_parts_mut(z_buffer_ptr, z_buffer_len);
    }

    //test----

    let mut vertex_clips;
    let mut vertex_processed;
    let vertex_clips_ptr;
    let vertex_processed_ptr;
    let model_heap;

    unsafe{
        let vertex_clips_len = vertices.len();
        let vertex_clips_bytes = vertex_clips_len * size_of::<VertexClip>();

        let vertex_processed_len = vertices.len();
        let vertex_processed_bytes = vertex_processed_len * size_of::<VertexProcessed>();

        let sum_byte = vertex_processed_bytes + vertex_clips_bytes;
        model_heap = HeapAlloc(process_heap, HEAP_ZERO_MEMORY, sum_byte);

        if model_heap.is_null() {
            ExitProcess(1);
        }

        vertex_clips_ptr = model_heap as *mut VertexClip;
        vertex_processed_ptr = (model_heap as *mut u8).add(vertex_clips_bytes) as *mut VertexProcessed;

        vertex_clips = slice::from_raw_parts_mut(vertex_clips_ptr, vertex_clips_len);
        vertex_processed = slice::from_raw_parts_mut(vertex_processed_ptr, vertex_processed_len);
    }

    {
        let mesh = Mesh_depr {
            vertices: &mut vertices,
            vertex_clips: &mut vertex_clips,
            vertex_processed: &mut vertex_processed,
            indices: &indices,
        };
        temp_registry = [mesh];
    }


    objects = [
        Object{
            mesh_id: 0,
            transform: Transform {
                x: -3.0,
                y: 0.0,
                z: 3.0,
                rotation_x: 0.0,
                rotation_y: 0.0,
                rotation_z: 0.0,
                scale_x: 1.0,
                scale_y: 1.0,
                scale_z: 1.0,
                matrix: Mat4::identity(),
                dirty: true,
            },
        },
        Object{
            mesh_id: 0,
            transform: Transform {
                x: 3.0,
                y: 0.0,
                z: 3.0,
                rotation_x: 0.0,
                rotation_y: 0.0,
                rotation_z: 0.0,
                scale_x: 1.0,
                scale_y: 1.0,
                scale_z: 1.0,
                matrix: Mat4::identity(),
                dirty: true,
            },
        }
    ];


    camera = Camera::new(90f32 * TO_RADIANS, 0.1, 100.0);
    camera.rebuild_projection(FRAME_WIDTH as f32 / FRAME_HEIGHT as f32);


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

    let mut msg: MSG;
    unsafe {
        ShowWindow(window, SW_SHOWNORMAL);
        msg = core::mem::zeroed()
    }


    let mut last_time = unsafe {GetTickCount64()};
    let mut delta_time:f32 = 0.0;
    let mut vertex_processed_to_render :[VertexProcessed;4]=[

    VertexProcessed{
        processed: false,
        screen_x: 0.0,
        screen_y: 0.0,
        inv_w: 0.0,
        r_over_w: 0.0,
        g_over_w: 0.0,
        b_over_w: 0.0,
    },
    VertexProcessed{
        processed: false,
        screen_x: 0.0,
        screen_y: 0.0,
        inv_w: 0.0,
        r_over_w: 0.0,
        g_over_w: 0.0,
        b_over_w: 0.0,
    },
    VertexProcessed{
        processed: false,
        screen_x: 0.0,
        screen_y: 0.0,
        inv_w: 0.0,
        r_over_w: 0.0,
        g_over_w: 0.0,
        b_over_w: 0.0,
    },
    VertexProcessed{
        processed: false,
        screen_x: 0.0,
        screen_y: 0.0,
        inv_w: 0.0,
        r_over_w: 0.0,
        g_over_w: 0.0,
        b_over_w: 0.0,
    }];


    loop {
        while {
            unsafe {
                PeekMessageW(
                    &mut msg,
                    null_mut(),
                    0,
                    0,
                    PM_REMOVE,
                )
            }
        } != 0
        {
            unsafe {
                if msg.message == WM_QUIT {
                    ExitProcess(msg.wParam as u32);
                }

                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }


        unsafe {
            if GetTickCount64() - last_time < 20 {
                Sleep(2);
                continue;
            }
        }

        delta_time = (unsafe{GetTickCount64()} - last_time) as f32 /1000.0;
        last_time = unsafe{GetTickCount64()};

        // work flow
        framebuffer.fill(Pixel::rgb(255, 255, 255));
        z_buffer.fill(0.0);

        unsafe {
            if GetAsyncKeyState(VK_D as i32) < 0 {
                camera.x += delta_time*10.0;
                camera.view_dirty = true;
            }else if GetAsyncKeyState(VK_A as i32) < 0 {
                camera.x -= delta_time*10.0;
                camera.view_dirty = true;
            }

            if GetAsyncKeyState(VK_W as i32) < 0 {
                camera.z += delta_time*10.0;
                camera.view_dirty = true;
            }else if GetAsyncKeyState(VK_S as i32) < 0 {
                camera.z -= delta_time*10.0;
                camera.view_dirty = true;
            }

            if GetAsyncKeyState(VK_SPACE as i32) < 0 {
                camera.y += delta_time*10.0;
                camera.view_dirty = true;
            }else if GetAsyncKeyState(VK_SHIFT as i32) < 0 {
                camera.y -= delta_time*10.0;
                camera.view_dirty = true;
            }

        }
        objects[0].transform.rotation_y += 10.0 * delta_time*TO_RADIANS;
        objects[0].transform.dirty = true;


        if camera.view_dirty || camera.projection_dirty {
            camera.rebuild_view();
            camera.rebuild_projection(FRAME_WIDTH as f32 / FRAME_HEIGHT as f32);
            camera.rebuild_view_projection()
        }
        for object_idx in 0..objects.len() {
            let object = &mut objects[object_idx];
            let mesh =&mut temp_registry[object.mesh_id];


            for i in 0..mesh.vertex_processed.len() {
                mesh.vertex_processed[i].processed = false;
            }

            if object.transform.dirty{
                object.rebuild_transform()
            }

            let final_transform = Mat4::mul( &camera.view_projection, &object.transform.matrix);

            for i in 0..mesh.vertices.len() {
                let vec = Vec4 {
                    x: mesh.vertices[i].x_model,
                    y: mesh.vertices[i].y_model,
                    z: mesh.vertices[i].z_model,
                    w: 1.0
                };
                let clip = Mat4::mul_vec4(&final_transform, vec);

                mesh.vertex_clips[i].x = clip.x;
                mesh.vertex_clips[i].y = clip.y;
                mesh.vertex_clips[i].z = clip.z;
                mesh.vertex_clips[i].w = clip.w;
                mesh.vertex_clips[i].r = mesh.vertices[i].r as f32 / 255.0;
                mesh.vertex_clips[i].g = mesh.vertices[i].g as f32 / 255.0;
                mesh.vertex_clips[i].b = mesh.vertices[i].b as f32 / 255.0;
            }

            for i in 0..mesh.indices.len() / 3 {
                let idx0 = mesh.indices[i * 3] as usize;
                let idx1 = mesh.indices[i * 3 + 1] as usize;
                let idx2 = mesh.indices[i * 3 + 2] as usize;

                let out_vertices =  near_plane_clip(
                    &mesh.vertex_clips[idx0], &mesh.vertex_clips[idx1], &mesh.vertex_clips[idx2],
                    mesh.vertex_processed,
                    idx0,
                    idx1,
                    idx2,
                    &mut vertex_processed_to_render,
                    0.1
                );

                if out_vertices == 0 {
                    continue;
                }
                rasterize(
                    &vertex_processed_to_render[0],
                    &vertex_processed_to_render[1],
                    &vertex_processed_to_render[2],
                    framebuffer,
                    z_buffer
                );
                if out_vertices == 4 {
                    rasterize(
                        &vertex_processed_to_render[0],
                        &vertex_processed_to_render[2],
                        &vertex_processed_to_render[3],
                        framebuffer,
                        z_buffer
                    );
                }

            }
        }

        present_frame(window, &bitmap_info, framebuffer, FRAME_WIDTH, FRAME_HEIGHT);
    }
}

fn process_vertex(vertex_clip: &VertexClip, vertex_processed:&mut VertexProcessed){
    vertex_processed.processed = true;

    vertex_processed.screen_x = (vertex_clip.x / vertex_clip.w + 1.0) * 0.5 * FRAME_WIDTH as f32;
    vertex_processed.screen_y = (1.0 - vertex_clip.y / vertex_clip.w) * 0.5 * FRAME_HEIGHT as f32;

    vertex_processed.inv_w = 1.0 / vertex_clip.w;

    vertex_processed.r_over_w = vertex_clip.r * vertex_processed.inv_w;
    vertex_processed.g_over_w = vertex_clip.g * vertex_processed.inv_w;
    vertex_processed.b_over_w = vertex_clip.b * vertex_processed.inv_w;
}

fn load_obj(path: PCWSTR) -> Option<Mesh> {
    let file = loader::read_entire_file(path)?;
    let text = file.text()?;

    let counts = objreader::count_obj(text)?;

    let mut mesh;

    {
        let model_vertices_bytes =
            counts.vertices
                .checked_mul(size_of::<VertexModel>())?;

        let indices_offset =
            align_up(
                model_vertices_bytes,
                align_of::<u32>(),
            )?;

        let indices_bytes =
            counts.indices
                .checked_mul(size_of::<u32>())?;

        let target_bytes =
            indices_offset
                .checked_add(indices_bytes)?;

        let heap = unsafe {
            HeapAlloc(
                GetProcessHeap(),
                HEAP_ZERO_MEMORY,
                target_bytes,
            )
        };

        if heap.is_null() {
            return None;
        }

        mesh = Mesh {
            heap,
            vertex_count: counts.vertices,
            index_count: counts.indices,
            indices_offset,
        };
    }

    {
        let mut load_view =
            mesh.load_view();

        parse_obj(
            text,
            &mut load_view,
        )?;
    }

    Some(mesh)
}

fn align_up(value: usize, alignment: usize) -> Option<usize> {
    let mask = alignment - 1;

    value
        .checked_add(mask)
        .map(|x| x & !mask)
}
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            unsafe {
                PostQuitMessage(0);
                0
            }
        },

        _ => {
            unsafe {
                DefWindowProcW(
                    hwnd,
                    msg,
                    wparam,
                    lparam,
                )
            }
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
    fn sinf(x: f32) -> f32;
    fn cosf(x: f32) -> f32;

}

