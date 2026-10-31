use core::ffi::c_void;
use core::slice;
use crate::{VertexClip, VertexProcessed};

struct RendererWorkspace {
    heap: *mut c_void,

    capacity: usize,

    vertex_processed_offset: usize,
}

impl RendererWorkspace {
    fn vertex_clip(&self) -> &[VertexClip] {
        unsafe {
            slice::from_raw_parts(
                self.heap as *const VertexClip,
                self.capacity)
        }
    }
    fn vertex_clip_mut(&mut self) -> &mut[VertexClip] {
        unsafe {
            slice::from_raw_parts_mut(
                self.heap as *mut VertexClip,
                self.capacity)
        }
    }
    fn vertex_processed(&self) -> &[VertexProcessed] {
        unsafe {
            slice::from_raw_parts(
                (self.heap as *const u8).add(
                    self.vertex_processed_offset) as *const VertexProcessed,
                self.capacity)
        }
    }
    fn vertex_processed_mut(&mut self) -> &mut[VertexProcessed] {
        unsafe {
            slice::from_raw_parts_mut(
                (self.heap as *mut u8).add(
                    self.vertex_processed_offset) as *mut VertexProcessed,
                self.capacity)
        }
    }
    pub fn get_view_workspace(
        &mut self,
        len: usize,
    ) -> Option<ViewWorkspace<'_>> {
        if len > self.capacity {
            return None;
        }

        unsafe {
            let base = self.heap as *mut u8;

            Some(ViewWorkspace {
                vertex_clip:
                slice::from_raw_parts_mut(
                    base as *mut VertexClip,
                    len,
                ),

                vertex_processed:
                slice::from_raw_parts_mut(
                    base.add(self.vertex_processed_offset)
                        as *mut VertexProcessed,
                    len,
                ),
            })
        }
    }
}

struct ViewWorkspace<'a>{
    pub vertex_clip: &'a mut [VertexClip],
    pub vertex_processed:&'a mut [VertexProcessed],
}