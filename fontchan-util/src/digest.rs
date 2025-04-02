use std::ops::Deref;
use std::path::Path;

use base64ct::Encoding;
use digest::generic_array::GenericArray;
use digest::OutputSizeUser;
use digest::Update;
use sha2::Digest;
use sha2::Sha512;

pub trait Hasher: Update {}

impl<T: Update> Hasher for T {}

type OutputArray = GenericArray<u8, <Sha512 as OutputSizeUser>::OutputSize>;

pub struct Digester(sha2::Sha512);

impl Digester {
    pub fn new() -> Self {
        Self(sha2::Sha512::new())
    }
    pub fn push(self, other: impl UpdateInto) -> Self {
        self.push_dyn(&other)
    }
    pub fn push_dyn(mut self, other: &dyn UpdateInto) -> Self {
        other.update_into(&mut self.0);
        self
    }
    pub fn push_file(mut self, file: impl AsRef<Path>) -> Self {
        let content = std::fs::read(file).unwrap();
        Update::update(&mut self.0, &content);
        self
    }
    pub fn base64_result(self) -> DigestString {
        self.0.finalize().into()
    }
    pub fn bytes_result(self) -> Vec<u8> {
        self.0.finalize().to_vec()
    }
}

pub trait UpdateInto {
    fn update_into(&self, hasher: &mut dyn Hasher);
}

impl<T> UpdateInto for T
where
    T: AsRef<[u8]>,
{
    fn update_into(&self, hasher: &mut dyn Hasher) {
        hasher.update(self.as_ref());
    }
}

#[derive(Debug)]
pub struct DigestString(Box<str>);

impl Deref for DigestString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<T> for DigestString
where
    T: Into<OutputArray>,
{
    fn from(s: T) -> Self {
        let mut str = base64ct::Base64Url::encode_string(s.into().as_slice());
        for b in unsafe { str.as_bytes_mut() } {
            if *b == b'_' {
                *b = b'+';
            }
        }
        Self(str.into_boxed_str())
    }
}

impl AsRef<str> for DigestString {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
