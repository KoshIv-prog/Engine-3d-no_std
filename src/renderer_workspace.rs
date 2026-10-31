use core::ffi::c_void;
use core::slice;
use crate::{VertexClip, VertexProcessed};

struct RendererWorkspace {
    heap: *mut c_void,

    vertex_clip_count: usize,
    vertex_processed_count: usize,

    vertex_clip_offset: usize,
}

impl RendererWorkspace {
    fn vertex_clip(&self) -> &[VertexClip] {
        unsafe {
            slice::from_raw_parts(
                self.heap as *const VertexClip,
                self.vertex_clip_count)
        }
    }
    fn vertex_clip_mut(&mut self) -> &mut[VertexClip] {
        unsafe {
            slice::from_raw_parts_mut(
                self.heap as *mut VertexClip,
                self.vertex_clip_count)
        }
    }
    fn vertex_processed(&self) -> &[VertexProcessed] {
        unsafe {
            slice::from_raw_parts(
                (self.heap as *const u8).add(
                    self.vertex_clip_offset) as *const VertexProcessed,
                self.vertex_clip_count)
        }
    }
    fn vertex_processed_mut(&mut self) -> &mut[VertexProcessed] {
        unsafe {
            slice::from_raw_parts_mut(
                (self.heap as *mut u8).add(
                    self.vertex_clip_offset) as *mut VertexProcessed,
                self.vertex_clip_count)
        }
    }
    pub fn get_view_workspace(&mut self) -> ViewWorkspace<'_> {
        unsafe {
            let base = self.heap as *mut u8;

            let vertex_clip =
                slice::from_raw_parts_mut(
                    base as *mut VertexClip,
                    self.vertex_clip_count,
                );

            let vertex_processed =
                slice::from_raw_parts_mut(
                    base.add(self.vertex_clip_offset)
                        as *mut VertexProcessed,
                    self.vertex_processed_count,
                );

            ViewWorkspace {
                vertex_clip,
                vertex_processed,
            }
        }
    }
}



struct ViewWorkspace<'a>{
    pub vertex_clip: &'a mut [VertexClip],
    pub vertex_processed:&'a mut [VertexProcessed],
}