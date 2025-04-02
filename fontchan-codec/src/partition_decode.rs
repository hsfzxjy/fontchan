#[cfg(feature = "has-std")]
pub use has_std::*;
#[cfg(feature = "has-std")]
mod has_std {
    use super::DecoderContext;

    #[derive(Clone)]
    pub struct VecWriter(Vec<u8>);

    impl VecWriter {
        pub fn new() -> Self {
            Self(Vec::new())
        }
        pub fn into_vec(self) -> Vec<u8> {
            self.0
        }
    }

    impl super::WriteBytes for VecWriter {
        fn write_bytes(mut self, bytes: &[u8]) -> Self {
            self.0.extend_from_slice(bytes);
            self
        }
    }

    #[derive(Copy, Clone)]
    pub struct CounterWriter(usize);

    impl CounterWriter {
        pub fn new() -> Self {
            Self(0)
        }
        pub fn value(&self) -> usize {
            self.0
        }
    }

    impl super::WriteBytes for CounterWriter {
        fn write_bytes(mut self, bytes: &[u8]) -> Self {
            self.0 += bytes.len();
            self
        }
    }

    pub struct StdContext<'a, W, T, FE, FS>
    where
        W: super::WriteBytes,
        FE: Fn(&T) -> &[u8],
        FS: for<'b> Fn(&'b T, &'b [u8]) -> &'b [u8],
    {
        pub writer: W,
        pub urange_data: &'a [u8],
        pub fid_data: &'a [u8],
        pub faces: &'a [T],
        pub ext_getter: FE,
        pub src_getter: FS,
    }

    impl<'a, W, T, FE, FS> StdContext<'a, W, T, FE, FS>
    where
        W: super::WriteBytes + Clone,
        FE: Fn(&T) -> &[u8],
        FS: for<'b> Fn(&'b T, &'b [u8]) -> &'b [u8],
    {
        pub fn decode(self) -> W {
            let w = self.writer.clone();
            super::Decoder::new(self).decode(w)
        }
    }

    impl<'a, W, T, FE, FS> DecoderContext for StdContext<'a, W, T, FE, FS>
    where
        W: super::WriteBytes,
        FE: Fn(&T) -> &[u8],
        FS: for<'b> Fn(&'b T, &'b [u8]) -> &'b [u8],
    {
        type Writer = W;

        fn urange_data(&self) -> super::Bytes<'_> {
            super::Bytes(self.urange_data)
        }

        fn fid_data(&self) -> super::Bytes<'_> {
            super::Bytes(self.fid_data)
        }

        fn font_face_count(&self) -> usize {
            self.faces.len()
        }

        fn write_font_face_ext(&self, idx: usize, writer: Self::Writer) -> Self::Writer {
            writer.write_bytes((self.ext_getter)(&self.faces[idx]))
        }

        fn write_font_face_src(
            &self,
            idx: usize,
            hash: &[u8],
            writer: Self::Writer,
        ) -> Self::Writer {
            writer.write_bytes((self.src_getter)(&self.faces[idx], hash))
        }
    }
}

pub trait DecoderContext {
    type Writer: WriteBytes;
    fn urange_data(&self) -> Bytes<'_>;
    fn fid_data(&self) -> Bytes<'_>;
    fn font_face_count(&self) -> usize;
    fn write_font_face_ext(&self, idx: usize, writer: Self::Writer) -> Self::Writer;
    fn write_font_face_src(&self, idx: usize, hash: &[u8], writer: Self::Writer) -> Self::Writer;
}

pub struct Decoder<C> {
    config: C,
}

impl<C: DecoderContext> Decoder<C> {
    pub fn new(config: C) -> Self {
        Self { config }
    }
    #[inline]
    fn write_font(&self, idx: usize, fid_data: &mut Bytes, mut out: C::Writer) -> C::Writer {
        let init = &self.config;
        let mut urange_list = init.urange_data();
        while !urange_list.is_empty() {
            let fid = fid_data.read_string();

            out = out.write_bytes(b"@font-face{");
            out = init.write_font_face_ext(idx, out);
            out = init.write_font_face_src(idx, fid, out);
            out = out.write_bytes(b"unicode-range:");
            let n_single = urange_list.read_varint();
            let mut prev = 0u32;
            for i in 0..n_single {
                if i != 0 {
                    out = out.write_bytes(b",");
                }
                let diff = urange_list.read_varint();
                let codepoint = prev.wrapping_add(diff);
                out = out.write_bytes(b"U+").write_codepoint(codepoint);
                prev = codepoint;
            }
            let n_ranges = urange_list.read_varint();
            if n_single != 0 && n_ranges != 0 {
                out = out.write_bytes(b",");
            }
            prev = 0;
            for i in 0..n_ranges {
                if i != 0 {
                    out = out.write_bytes(b",");
                }
                let start = prev.wrapping_add(urange_list.read_varint());
                let end = start.wrapping_add(urange_list.read_varint());
                out = out
                    .write_bytes(b"U+")
                    .write_codepoint(start)
                    .write_bytes(b"-")
                    .write_codepoint(end);
                prev = end;
            }
            out = out.write_bytes(b";}");
        }
        out
    }

    pub fn decode(&self, mut out: C::Writer) -> C::Writer {
        let mut fid_data = self.config.fid_data();
        for idx in 0..self.config.font_face_count() {
            out = self.write_font(idx, &mut fid_data, out);
        }
        out
    }
}

pub trait WriteBytes: Sized {
    #[must_use]
    fn write_bytes(self, bytes: &[u8]) -> Self;
}

pub trait WriteBytesExt: WriteBytes {
    #[must_use]
    fn write_codepoint(mut self, mut codepoint: u32) -> Self {
        let mut buf = [0u8; 8];
        let mut len = 0;
        const DIGITS: &[u8] = b"0123456789abcdef";
        while codepoint != 0 {
            unsafe {
                *buf.get_unchecked_mut(len) = *DIGITS.get_unchecked((codepoint & 0xf) as usize);
            }
            codepoint >>= 4;
            len += 1;
        }
        if len == 0 {
            len = 1
        }
        if buf.len() >= len {
            buf[..len].reverse();
            self = self.write_bytes(&buf[..len])
        } else {
            crate::core::unreachable()
        }
        self
    }
}

impl<W: WriteBytes> WriteBytesExt for W {}

#[derive(Copy, Clone)]
pub struct Bytes<'a>(&'a [u8]);

impl<'a> Bytes<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self(data)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> Bytes<'a> {
    pub(crate) fn read_varint(&mut self) -> u32 {
        let mut result: u32 = 0;
        let mut shift = 0;
        const DROP_MSB: u8 = 127;
        const MSB: u8 = 128;

        let mut success = false;
        let mut b: u8;
        let mut src = self.0;
        while !src.is_empty() {
            (b, src) = crate::bytes::split_at_first(src);
            let msb_dropped = b & DROP_MSB;
            result |= (msb_dropped as u32) << shift;
            shift += 7;

            if b & MSB == 0 || shift > (9 * 7) {
                success = b & MSB == 0;
                break;
            }
        }

        if !success {
            crate::core::unreachable()
        }

        self.0 = src;
        result
    }

    pub(crate) fn read_string(&mut self) -> &'a [u8] {
        let len = self.read_varint() as usize;
        let (result, src) = crate::bytes::split_at(self.0, len);
        self.0 = src;
        result
    }
}
