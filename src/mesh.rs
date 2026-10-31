use core::ffi::c_void;
use crate::VertexModel;

struct Mesh {
    heap: *mut c_void,

    vertex_count: usize,
    index_count: usize,

    indices_offset: usize,
}

impl Mesh {
    fn vertices(&self) -> &[VertexModel] {
        unsafe {
            core::slice::from_raw_parts(
                self.heap as *const VertexModel,
                self.vertex_count,
            )
        }
    }

    fn vertices_mut(&mut self) -> &mut [VertexModel] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self.heap as *mut VertexModel,
                self.vertex_count,
            )
        }
    }

    fn indices(&self) -> &[u32] {
        unsafe {
            core::slice::from_raw_parts(
                (self.heap as *const u8)
                    .add(self.indices_offset)
                    as *const u32,
                self.index_count,
            )
        }
    }

    fn indices_mut(&mut self) -> &mut [u32] {
        unsafe {
            core::slice::from_raw_parts_mut(
                (self.heap as *mut u8)
                    .add(self.indices_offset)
                    as *mut u32,
                self.index_count,
            )
        }
    }

    fn mesh_view(&self) -> MeshView<'_> {
        MeshView {
            vertices: self.vertices(),
            indices: self.indices(),
        }
    }
}

struct MeshView<'a> {
    vertices: &'a [VertexModel],
    indices: &'a [u32],
}