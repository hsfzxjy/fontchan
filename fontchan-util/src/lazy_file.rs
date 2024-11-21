use std::{
    io,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use crate::{Hasher, UpdateInto};

#[derive(Debug, Default, serde::Deserialize)]
#[serde(transparent)]
pub struct LazyFile {
    path: PathBuf,
    #[serde(skip)]
    content: OnceLock<Result<Vec<u8>, io::Error>>,
    #[serde(skip)]
    digest: OnceLock<Result<Vec<u8>, io::Error>>,
}

impl LazyFile {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            ..Default::default()
        }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn content(&self) -> Result<&[u8], &io::Error> {
        self.content
            .get_or_init(|| std::fs::read(&self.path))
            .as_ref()
            .map(Vec::as_slice)
    }
    pub fn digest(&self) -> Result<&[u8], &io::Error> {
        let content = self.content()?;
        self.digest
            .get_or_init(|| Ok(crate::digest::Digester::new().push(content).bytes_result()))
            .as_ref()
            .map(Vec::as_slice)
    }
}

impl<T> From<T> for LazyFile
where
    T: Into<PathBuf>,
{
    fn from(path: T) -> Self {
        Self::new(path.into())
    }
}

impl UpdateInto for &LazyFile {
    fn update_into(&self, hasher: &mut dyn Hasher) {
        hasher.update(self.digest().unwrap());
    }
}
