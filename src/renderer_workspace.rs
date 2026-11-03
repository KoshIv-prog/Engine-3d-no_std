use crate::{VertexClip, VertexProcessed, align_up};
use core::ffi::c_void;
use core::fmt::Error;
use core::ptr::null_mut;
use core::slice;
use windows_sys::Win32::System::Memory::{GetProcessHeap, HeapAlloc, HeapFree};

pub struct RendererWorkspace {
    heap: *mut c_void,

    capacity: usize,

    vertex_processed_offset: usize,
}

impl Drop for RendererWorkspace {
    fn drop(&mut self) {
        unsafe {
            HeapFree(GetProcessHeap(), 0, self.heap);
        }
    }
}

impl RendererWorkspace {

    pub fn new() -> Self {
        RendererWorkspace{
            heap: null_mut(),
            capacity: 0,
            vertex_processed_offset:0
        }
    }
    /*fn vertex_clip(&self) -> &[VertexClip] {
        unsafe {
            slice::from_raw_parts(
                self.heap as *const VertexClip,
                self.capacity)
        }
    }*/
    /*fn vertex_clip_mut(&mut self) -> &mut[VertexClip] {
        unsafe {
            slice::from_raw_parts_mut(
                self.heap as *mut VertexClip,
                self.capacity)
        }
    }*/
    /*fn vertex_processed(&self) -> &[VertexProcessed] {
        unsafe {
            slice::from_raw_parts(
                (self.heap as *const u8).add(
                    self.vertex_processed_offset) as *const VertexProcessed,
                self.capacity)
        }
    }*/
    /*fn vertex_processed_mut(&mut self) -> &mut[VertexProcessed] {
        unsafe {
            slice::from_raw_parts_mut(
                (self.heap as *mut u8).add(
                    self.vertex_processed_offset) as *mut VertexProcessed,
                self.capacity)
        }
    }*/
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
    pub fn ensure_capacity(&mut self, len: usize) -> Result<(),Error>{
        if len <= self.capacity {
            return Ok(());
        }
        let old_heap = self.heap;

        let target_capacity;
        if self.capacity == 0{
            target_capacity = len;
        }else{
            let doubled = match self.capacity.checked_mul(2) {
                Some(v) => v,
                None => return Err(Default::default()),
            };

            if len > doubled {
                target_capacity = len;
            } else {
                target_capacity = doubled;
            }
        }

        //alloc
        let vertex_clip_bytes =match target_capacity.checked_mul(size_of::<VertexClip>()){
            Some(v) => v,
            None => return Err(Default::default())
        };

        let vertex_processed_bytes =match target_capacity.checked_mul(size_of::<VertexProcessed>()){
            Some(v) => v,
            None => return Err(Default::default())
        };

        let vertex_processed_offset =match align_up(vertex_clip_bytes, align_of::<VertexProcessed>()){
            Some(v) => v,
            None => return Err(Default::default())
        };

        let targeted_size =match vertex_processed_offset.checked_add(vertex_processed_bytes){
            Some(v) => v,
            None => return Err(Default::default())
        };

        let mut new_heap;
        unsafe {
            new_heap = HeapAlloc(GetProcessHeap(), 0, targeted_size);//без heap_zero_memody бо на початку проходу ми всеодно перераховуємо все
        }
        if new_heap.is_null(){
            return Err(Default::default());
        }

        if !self.heap.is_null() {
            unsafe {
                HeapFree(GetProcessHeap(), 0, old_heap);
            }
        }

        self.capacity = target_capacity;
        self.vertex_processed_offset = vertex_processed_offset;
        self.heap = new_heap;

        Ok(())
    }
}

pub struct ViewWorkspace<'a>{
    pub vertex_clip: &'a mut [VertexClip],
    pub vertex_processed:&'a mut [VertexProcessed],
}