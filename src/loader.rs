use core::ffi::c_void;
use core::ptr::{null, null_mut};
use core::slice;

use windows_sys::core::PCWSTR;

use windows_sys::Win32::Foundation::{
    CloseHandle,
    GENERIC_READ,
    INVALID_HANDLE_VALUE,
};

use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW,
    FILE_ATTRIBUTE_NORMAL,
    FILE_SHARE_READ,
    GetFileSizeEx,
    OPEN_EXISTING,
    ReadFile,
};

use windows_sys::Win32::System::Memory::{
    GetProcessHeap,
    HEAP_ZERO_MEMORY,
    HeapAlloc,
    HeapFree,
};

pub struct FileBuffer {
    ptr: *mut u8,
    len: usize,
}

impl FileBuffer {
    fn bytes(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                self.ptr,
                self.len,
            )
        }
    }

    pub fn text(&self) -> Option<&str> {
        core::str::from_utf8(
            self.bytes()
        ).ok()
    }
}

impl Drop for FileBuffer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                HeapFree(
                    GetProcessHeap(),
                    0,
                    self.ptr as *mut c_void,
                );
            }
        }
    }
}

pub fn read_entire_file(
    path: PCWSTR,
) -> Option<FileBuffer> {
    unsafe {
        let file = CreateFileW(
            path,
            GENERIC_READ,
            FILE_SHARE_READ,
            null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            null_mut(),
        );

        if file == INVALID_HANDLE_VALUE {
            return None;
        }

        let mut file_size: i64 = 0;

        if GetFileSizeEx(
            file,
            &mut file_size,
        ) == 0 {
            CloseHandle(file);
            return None;
        }

        if file_size <= 0
            || file_size > u32::MAX as i64
        {
            CloseHandle(file);
            return None;
        }

        let len = file_size as usize;

        let ptr = HeapAlloc(
            GetProcessHeap(),
            HEAP_ZERO_MEMORY,//todo check
            len,
        ) as *mut u8;

        if ptr.is_null() {
            CloseHandle(file);
            return None;
        }

        let mut bytes_read = 0u32;

        let result = ReadFile(
            file,
            ptr,
            len as u32,
            &mut bytes_read,
            null_mut(),
        );

        CloseHandle(file);

        if result == 0
            || bytes_read as usize != len
        {
            HeapFree(
                GetProcessHeap(),
                0,
                ptr as *mut c_void,
            );

            return None;
        }


        Some(FileBuffer {
            ptr,
            len,

        })
    }
}