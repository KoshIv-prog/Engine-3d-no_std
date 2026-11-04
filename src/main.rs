#![no_std]
#![no_main]
#![windows_subsystem = "windows"]
//extern crate alloc;

mod math;
mod rasterizator;
mod clipping;
mod loader;
mod objreader;
mod mesh;
mod renderer_workspace;
pub mod camera;

use crate::MouseMode::{Captured, Free};
use crate::camera::Camera;
use crate::clipping::near_plane_clip;
use crate::math::{Mat4, Vec3, Vec4};
use crate::mesh::Mesh;
use crate::objreader::parse_obj;
use crate::rasterizator::rasterize;
use crate::renderer_workspace::RendererWorkspace;
use core::panic::PanicInfo;
use core::ptr::{copy, null, null_mut, write};
use core::slice;
use windows_sys::Win32::Foundation::{FALSE, HWND, LPARAM, LRESULT, POINT, RECT, TRUE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{BITMAPINFO, BITMAPINFOHEADER, BI_RGB, COLORONCOLOR, ClientToScreen, DIB_RGB_COLORS, GetDC, RGBQUAD, ReleaseDC, SRCCOPY, SetStretchBltMode, StretchDIBits};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{GetProcessHeap, HEAP_ZERO_MEMORY, HeapAlloc};
use windows_sys::Win32::System::SystemInformation::GetTickCount64;
use windows_sys::Win32::System::Threading::{ExitProcess, Sleep};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_A, VK_CONTROL, VK_D, VK_ESCAPE, VK_S, VK_SHIFT, VK_SPACE, VK_W};
use windows_sys::Win32::UI::Input::{GetRawInputData, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RID_INPUT, RIM_TYPEMOUSE, RegisterRawInputDevices};
use windows_sys::Win32::UI::WindowsAndMessaging::{AdjustWindowRectEx, CREATESTRUCTW, ClipCursor, CreateWindowExW, DefWindowProcW, DispatchMessageW, GWLP_USERDATA, GetClientRect, GetWindowLongPtrW, MSG, PM_REMOVE, PeekMessageW, PostQuitMessage, RegisterClassExW, SW_SHOWNORMAL, SetWindowLongPtrW, ShowCursor, ShowWindow, TranslateMessage, WM_DESTROY, WM_INPUT, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS, WM_MOUSEMOVE, WM_NCCREATE, WM_QUIT, WM_SETFOCUS, WNDCLASSEXW, WS_OVERLAPPEDWINDOW};
use windows_sys::core::PCWSTR;
use windows_sys::w;

const FRAME_HEIGHT: i32 = 720; //9 * 10;
const FRAME_WIDTH: i32 = 1280; //16*10;

const SCREEN_HEIGHT: i32 = 720;
const SCREEN_WIDTH :i32 = 1280;
const TO_RADIANS: f32 = 0.017453292519943295;
const MOVE_SPEED:f32 = 10.0;
const MAX_PITCH: f32 = 89.0 * TO_RADIANS;

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

    view_x:f32,
    view_y:f32,
    view_z:f32,

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

struct WindowState{
    input : InputState,
    mouse_mode: MouseMode,
}

struct InputState{
    w: bool,
    a: bool,
    s: bool,
    d: bool,

    space: bool,
    shift: bool,

    mouse_delta_x: i32,
    mouse_delta_y: i32,

    mouse_x: i32,
    mouse_y: i32,

    focused: bool,
    toggle_mouse_mode:bool,
}
#[derive(Clone, Copy, PartialEq)]
enum MouseMode {
    Free,
    Captured,
}

#[unsafe(no_mangle)]
pub extern "system" fn mainCRTStartup() -> ! {
    let mut temp_registry:[Mesh;1];
    let mut render_workspace = RendererWorkspace::new();

    let mut objects;
    let mut camera;

    let process_heap = unsafe { GetProcessHeap() };

    let window_state_heap;
    let window_state_ptr;
    {
        let window_state_bytes = size_of::<WindowState>();
        unsafe {
            window_state_heap = HeapAlloc(process_heap, HEAP_ZERO_MEMORY, window_state_bytes);
            if window_state_heap.is_null() {
                ExitProcess(1);
            }
        }
        window_state_ptr = window_state_heap as *mut WindowState;
        unsafe {
            write(window_state_ptr, WindowState {
                input: InputState {
                    w: false,
                    a: false,
                    s: false,
                    d: false,
                    space: false,
                    shift: false,
                    mouse_delta_x: 0,
                    mouse_delta_y: 0,
                    mouse_x: 0,
                    mouse_y: 0,
                    focused: false,
                    toggle_mouse_mode: false,
                },
                mouse_mode: MouseMode::Captured,
            });
        }
    }



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
            window_state_heap,
        );
        if window.is_null() {
            ExitProcess(1)
        }
    }

    let mouse = RAWINPUTDEVICE {
        usUsagePage: 0x01, // Generic Desktop Controls
        usUsage: 0x02,     // Mouse
        dwFlags: 0,
        hwndTarget: window,
    };

    unsafe {
        if RegisterRawInputDevices(
            &mouse,
            1,
            size_of::<RAWINPUTDEVICE>() as u32,
        ) == 0
        {
            ExitProcess(1);
        }
    }



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


    temp_registry = match load_obj(w!("Arch.obj")){
        None => unsafe{ExitProcess(1);}
        Some(mesh) => [mesh]
    };
    match render_workspace.ensure_capacity(temp_registry[0].vertex_count){
        Ok(_) => {}
        Err(_) => {unsafe {ExitProcess(1);}}
    };


    objects = [
        Object {
            mesh_id: 0,
            transform: Transform {
                x: 0.0,
                y: -1.2,
                z: 3.0,
                rotation_x: 0.0,
                rotation_y: 180.0 * TO_RADIANS,
                rotation_z: 0.0,
                scale_x: 1.0,
                scale_y: 1.0,
                scale_z: 1.0,
                matrix: Mat4::identity(),
                dirty: true,
            },
        },/*
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
        */
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
    let mut delta_time:f32;
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

        {
            let mode_to_set = {
                let state = unsafe { &mut *window_state_ptr };

                if state.input.toggle_mouse_mode {
                    state.input.toggle_mouse_mode = false;

                    Some(
                        if state.mouse_mode == MouseMode::Captured {
                            MouseMode::Free
                        } else {
                            MouseMode::Captured
                        }
                    )
                } else {
                    None
                }
            };

            if let Some(mode) = mode_to_set {
                set_mouse_mode(window, mode);

                let state = unsafe { &mut *window_state_ptr };

                state.mouse_mode = mode;
                state.input.mouse_delta_x = 0;
                state.input.mouse_delta_y = 0;
            }
        }

        {
            let window_state= unsafe { &mut *window_state_ptr };


            let dx = window_state.input.mouse_delta_x;
            let dy = window_state.input.mouse_delta_y;

            window_state.input.mouse_delta_x = 0;
            window_state.input.mouse_delta_y = 0;

            if window_state.input.d {
                camera.move_relative_strafe(delta_time * MOVE_SPEED);
            } else if window_state.input.a {
                camera.move_relative_strafe(-delta_time * MOVE_SPEED);
            }

            if window_state.input.w {
                camera.move_relative_forward(delta_time * MOVE_SPEED);
            } else if window_state.input.s {
                camera.move_relative_forward(-delta_time * MOVE_SPEED);
            }

            if window_state.input.space {
                camera.y += delta_time * MOVE_SPEED;
                camera.view_dirty = true;
            } else if window_state.input.shift {
                camera.y -= delta_time * MOVE_SPEED;
                camera.view_dirty = true;
            }

            if window_state.mouse_mode == MouseMode::Captured {
                if (dx != 0 || dy != 0) && window_state.input.focused {
                    camera.yaw +=
                        dx as f32 * 0.002;
                    camera.pitch +=
                        dy as f32 * 0.002;
                    if camera.pitch > MAX_PITCH {
                        camera.pitch = MAX_PITCH;
                    } else if camera.pitch < -MAX_PITCH {
                        camera.pitch = -MAX_PITCH;
                    }
                    camera.yaw = camera.yaw % (360.0 * TO_RADIANS);
                    camera.view_dirty = true;
                }
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
            let mesh =&mut temp_registry[object.mesh_id].mesh_view();

            let view_workspace = match render_workspace.get_view_workspace(mesh.vertices.len()){
                Some(workspace) => workspace,
                None => continue,
            };

            for i in 0..view_workspace.vertex_processed.len() {
                view_workspace.vertex_processed[i].processed = false;
            }

            if object.transform.dirty{
                object.rebuild_transform()
            }

            let model_view = Mat4::mul(&camera.view, &object.transform.matrix);

            for i in 0..mesh.vertices.len() {
                let vec = Vec4 {
                    x: mesh.vertices[i].x_model,
                    y: mesh.vertices[i].y_model,
                    z: mesh.vertices[i].z_model,
                    w: 1.0
                };
                let view = Mat4::mul_vec4(&model_view, vec);
                view_workspace.vertex_clip[i].view_x = view.x;
                view_workspace.vertex_clip[i].view_y = view.y;
                view_workspace.vertex_clip[i].view_z = view.z;

                let clip = Mat4::mul_vec4(&camera.projection, view);
                view_workspace.vertex_clip[i].x = clip.x;
                view_workspace.vertex_clip[i].y = clip.y;
                view_workspace.vertex_clip[i].z = clip.z;
                view_workspace.vertex_clip[i].w = clip.w;
                view_workspace.vertex_clip[i].r = mesh.vertices[i].r as f32 / 255.0;
                view_workspace.vertex_clip[i].g = mesh.vertices[i].g as f32 / 255.0;
                view_workspace.vertex_clip[i].b = mesh.vertices[i].b as f32 / 255.0;
            }

            for i in 0..mesh.indices.len() / 3 {
                let idx0 = mesh.indices[i * 3] as usize;
                let idx1 = mesh.indices[i * 3 + 1] as usize;
                let idx2 = mesh.indices[i * 3 + 2] as usize;

                let normal = face_normal(
                    &view_workspace.vertex_clip[idx0],
                    &view_workspace.vertex_clip[idx1],
                    &view_workspace.vertex_clip[idx2]
                );

                let show = is_font_face(
                    &view_workspace.vertex_clip[idx0],
                    &normal
                );

                if !show {
                    continue;
                }

                let out_vertices =  near_plane_clip(
                    &view_workspace.vertex_clip[idx0], &view_workspace.vertex_clip[idx1], &view_workspace.vertex_clip[idx2],
                    view_workspace.vertex_processed,
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

fn face_normal(v0: &VertexClip, v1: &VertexClip, v2: &VertexClip) -> Vec3{
    let e1_x = v1.view_x - v0.view_x;
    let e1_y = v1.view_y - v0.view_y;
    let e1_z = v1.view_z - v0.view_z;

    let e2_x = v2.view_x - v0.view_x;
    let e2_y = v2.view_y - v0.view_y;
    let e2_z = v2.view_z - v0.view_z;

    Vec3{
        x: e1_y * e2_z - e1_z * e2_y,
        y: e1_z * e2_x - e1_x * e2_z,
        z: e1_x * e2_y - e1_y * e2_x
    }
}
fn is_font_face(vertex_clip_idx0: &VertexClip, triangle_normal: &Vec3) -> bool{
    let to_camera_x = -vertex_clip_idx0.view_x;
    let to_camera_y = -vertex_clip_idx0.view_y;
    let to_camera_z = -vertex_clip_idx0.view_z;

    let dot =
        triangle_normal.x * to_camera_x
            + triangle_normal.y * to_camera_y
            + triangle_normal.z * to_camera_z;
    dot > 0.0

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
    if msg == WM_NCCREATE {
        let create =unsafe{
            &*(lparam as *const CREATESTRUCTW)};

        let state =
            create.lpCreateParams as *mut WindowState;

        unsafe {
            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                state as isize,
            );
        }
    }

    let state_ptr = unsafe{
        GetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
        )} as *mut WindowState;

    if state_ptr.is_null() {
        return unsafe{DefWindowProcW(hwnd, msg, wparam, lparam)};
    }

    match msg {
        WM_KILLFOCUS =>{
            let state = unsafe{ &mut *state_ptr};
            state.input.a = false;
            state.input.w = false;
            state.input.s = false;
            state.input.d = false;
            state.input.shift = false;
            state.input.space = false;
            state.input.focused = false;

            if state.mouse_mode == Captured {
                unsafe{
                    ClipCursor(null());
                    ShowCursor(TRUE);
                }
                state.input.mouse_delta_y = 0;
                state.input.mouse_delta_x = 0;
            }

            unsafe {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        },
        WM_SETFOCUS => {
            let state = unsafe{ &mut *state_ptr};
            state.input.focused = true;

            if state.mouse_mode == Captured {
                let mut bounds = RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };

                unsafe {
                    GetClientRect(hwnd, &mut bounds);
                }

                let mut top_left =POINT{x: bounds.left, y: bounds.top};
                let mut bottom_right =POINT{x: bounds.right, y: bounds.bottom};
                unsafe {
                    ClientToScreen(hwnd, &mut top_left);
                    ClientToScreen(hwnd, &mut bottom_right);
                }
                bounds.top = top_left.y;
                bounds.left = top_left.x;
                bounds.bottom = bottom_right.y;
                bounds.right = bottom_right.x;
                unsafe {
                    ClipCursor(&bounds);
                    ShowCursor(FALSE);
                }

                state.input.mouse_delta_y = 0;
                state.input.mouse_delta_x = 0;
            }

            unsafe {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        },
        WM_INPUT => {
            let state = unsafe{ &mut *state_ptr};
            let mut raw: RAWINPUT =
                unsafe{core::mem::zeroed()};

            let mut size =
                size_of::<RAWINPUT>() as u32;

            let result = unsafe {
                GetRawInputData(
                    lparam as _,
                    RID_INPUT,
                    &mut raw as *mut RAWINPUT as _,
                    &mut size,
                    size_of::<RAWINPUTHEADER>() as u32,
                )};


            if result != u32::MAX
                && raw.header.dwType == RIM_TYPEMOUSE
            {
                let mouse = unsafe{raw.data.mouse};

                state.input.mouse_delta_x += mouse.lLastX;
                state.input.mouse_delta_y += mouse.lLastY;
            }

            unsafe {
                DefWindowProcW(
                    hwnd,
                    msg,
                    wparam,
                    lparam,
                )
            }

        },
        WM_MOUSEMOVE => {
            let state = &mut *state_ptr;

            let x =
                (lparam as u32 & 0xFFFF) as i16 as i32;

            let y =
                ((lparam as u32 >> 16) & 0xFFFF)
                    as i16 as i32;

            state.input.mouse_x = x;
            state.input.mouse_y = y;

            unsafe {
                DefWindowProcW(
                    hwnd,
                    msg,
                    wparam,
                    lparam,
                )
            }
        },
        WM_KEYDOWN => {
            let state = unsafe{ &mut *state_ptr};
            if !state.input.focused{
                return 0;
            }
            let first_press =
                (lparam & (1 << 30)) == 0;

            if first_press {
                match wparam as u16 {
                    VK_CONTROL => {
                        state.input.toggle_mouse_mode = true;
                    }

                    _ => {}
                }
            }
            match wparam as u16 {
                VK_W => state.input.w = true,
                VK_A => state.input.a = true,
                VK_S => state.input.s = true,
                VK_D => state.input.d = true,
                VK_SPACE => state.input.space = true,
                VK_SHIFT => state.input.shift = true,
                VK_ESCAPE => {
                    unsafe {
                        PostQuitMessage(0);
                        return  0
                    }
                }
                _ => {}
            }

            0
        },

        WM_KEYUP => {
            let state = unsafe{ &mut *state_ptr};
            if !state.input.focused{
                return 0;
            }
            match wparam as u16 {
                VK_W => state.input.w = false,
                VK_A => state.input.a = false,
                VK_S => state.input.s = false,
                VK_D => state.input.d = false,
                VK_SPACE => state.input.space = false,
                VK_SHIFT => state.input.shift = false,
                _ => {}
            }
            0
        },

        WM_DESTROY => {
            unsafe {
                PostQuitMessage(0);
                0
            }
        }

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
fn set_mouse_mode(
    hwnd: HWND,
    mode: MouseMode,
){
    let state_ptr = unsafe{
        GetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
        )} as *mut WindowState;

    if state_ptr.is_null() {
        return;
    }
    let state = unsafe{ &mut *state_ptr};
    match mode {
        MouseMode::Captured => unsafe {
            let mut bounds = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };


            GetClientRect(hwnd, &mut bounds);

            let mut top_left =POINT{x: bounds.left, y: bounds.top};
            let mut bottom_right =POINT{x: bounds.right, y: bounds.bottom};
            ClientToScreen(hwnd, &mut top_left);
            ClientToScreen(hwnd, &mut bottom_right);

            bounds.top = top_left.y;
            bounds.left = top_left.x;
            bounds.bottom = bottom_right.y;
            bounds.right = bottom_right.x;
            ClipCursor(&bounds);
            ShowCursor(FALSE);
            state.mouse_mode = Captured;

            state.input.mouse_delta_y = 0;
            state.input.mouse_delta_x = 0;

        }
        MouseMode::Free => unsafe {
            ClipCursor(null());
            ShowCursor(TRUE);
            state.mouse_mode = Free;
            state.input.mouse_delta_y = 0;
            state.input.mouse_delta_x = 0;
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
    fn sqrtf(x: f32) -> f32;
    fn ucrt(x: f32) -> f32;
}

