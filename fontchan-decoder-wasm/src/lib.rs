#![no_std]

use fontchan_codec::{Bytes, Decoder, DecoderContext, WriteBytes};

const DUMMY: usize = 42;

#[no_mangle]
pub static mut URANGE_START: usize = DUMMY;

#[no_mangle]
pub static mut URANGE_LEN: usize = DUMMY;

#[no_mangle]
pub static mut FID_START: usize = DUMMY;

#[no_mangle]
pub static mut FID_LEN: usize = DUMMY;

#[no_mangle]
pub static mut HEAP_START: usize = DUMMY;

#[no_mangle]
pub extern "C" fn init_writer() -> *mut u8 {
    unsafe { HEAP_START as *mut u8 }
}

#[no_mangle]
pub unsafe extern "C" fn decode_css(font_face_count: usize, writer: *mut u8) -> *mut u8 {
    let init = Config {
        urange_data: Bytes::new(core::slice::from_raw_parts(
            URANGE_START as *const u8,
            URANGE_LEN,
        )),
        fid_data: Bytes::new(core::slice::from_raw_parts(FID_START as *const u8, FID_LEN)),
        font_face_count,
    };
    let decoder = Decoder::new(init);
    let writer = UnsafeWriter::new(writer);
    decoder.decode(writer).data
}

#[allow(dead_code)]
extern "C" {
    fn js_write_font_face_ext(fontid: usize, writer: *mut u8) -> *mut u8;
    fn js_write_font_face_src(
        fontid: usize,
        hash: *const u8,
        hash_len: usize,
        writer: *mut u8,
    ) -> *mut u8;
}

struct Config {
    urange_data: Bytes<'static>,
    fid_data: Bytes<'static>,
    font_face_count: usize,
}

impl DecoderContext for Config {
    type Writer = UnsafeWriter;

    fn urange_data(&self) -> Bytes<'_> {
        self.urange_data
    }

    fn fid_data(&self) -> Bytes<'_> {
        self.fid_data
    }

    fn font_face_count(&self) -> usize {
        self.font_face_count
    }

    fn write_font_face_ext(&self, idx: usize, writer: Self::Writer) -> Self::Writer {
        let writer = unsafe { js_write_font_face_ext(idx, writer.data) };
        UnsafeWriter::new(writer)
    }

    fn write_font_face_src(&self, idx: usize, hash: &[u8], writer: Self::Writer) -> Self::Writer {
        let writer = unsafe { js_write_font_face_src(idx, hash.as_ptr(), hash.len(), writer.data) };
        UnsafeWriter::new(writer)
    }
}

struct UnsafeWriter {
    data: *mut u8,
}

impl<'a> UnsafeWriter {
    pub fn new(data: *mut u8) -> Self {
        Self { data }
    }
}

impl WriteBytes for UnsafeWriter {
    #[inline]
    fn write_bytes(mut self, bytes: &[u8]) -> Self {
        let len = bytes.len();
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), self.data, len);
            self.data = self.data.add(len);
        }
        self
    }
}
