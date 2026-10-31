use core::ffi::c_void;
use core::slice;
use windows_sys::Win32::System::Memory::{GetProcessHeap, HeapFree};
use crate::VertexModel;

pub struct Mesh {
    pub heap: *mut c_void,

    pub vertex_count: usize,
    pub index_count: usize,

    pub indices_offset: usize,
}
impl Drop for Mesh {
    fn drop(&mut self) {
        if !self.heap.is_null() {
            unsafe {
                HeapFree(
                    GetProcessHeap(),
                    0,
                    self.heap,
                );
            }
        }
    }
}
impl Mesh {
    pub fn vertices(&self) -> &[VertexModel] {
        unsafe {
            core::slice::from_raw_parts(
                self.heap as *const VertexModel,
                self.vertex_count,
            )
        }
    }

    pub fn vertices_mut(&mut self) -> &mut [VertexModel] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self.heap as *mut VertexModel,
                self.vertex_count,
            )
        }
    }

    pub fn indices(&self) -> &[u32] {
        unsafe {
            core::slice::from_raw_parts(
                (self.heap as *const u8)
                    .add(self.indices_offset)
                    as *const u32,
                self.index_count,
            )
        }
    }

    pub fn indices_mut(&mut self) -> &mut [u32] {
        unsafe {
            core::slice::from_raw_parts_mut(
                (self.heap as *mut u8)
                    .add(self.indices_offset)
                    as *mut u32,
                self.index_count,
            )
        }
    }

    pub fn mesh_view(&self) -> MeshView<'_> {
        MeshView {
            vertices: self.vertices(),
            indices: self.indices(),
        }
    }
    pub fn load_view(&mut self) -> MeshLoadView<'_> {
        unsafe {
            let base = self.heap as *mut u8;

            MeshLoadView {
                vertices: slice::from_raw_parts_mut(
                    base as *mut VertexModel,
                    self.vertex_count,
                ),

                indices: slice::from_raw_parts_mut(
                    base.add(self.indices_offset) as *mut u32,
                    self.index_count,
                ),
            }
        }
    }
}

pub struct MeshView<'a> {
    pub vertices: &'a [VertexModel],
    pub indices: &'a [u32],
}
pub struct MeshLoadView<'a> {
    pub vertices: &'a mut [VertexModel],
    pub indices: &'a mut [u32],
}