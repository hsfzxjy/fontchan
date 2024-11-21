use std::{ffi::OsStr, path::PathBuf};

use anyhow::{bail, Result};
use tempfile::{NamedTempFile, TempPath};

#[derive(Debug)]
pub struct AtomicPath {
    real: PathBuf,
}

impl AtomicPath {
    pub fn new(real: PathBuf) -> Self {
        Self { real }
    }
    pub fn into_writable(self) -> Result<WritableAtomicPath> {
        let Some(parent_dir) = self.real.parent() else {
            bail!("Parent directory not found for path: {:?}", self.real);
        };
        let temp_file = NamedTempFile::with_prefix_in("__fontchan", parent_dir)?;
        Ok(WritableAtomicPath {
            real: self.real,
            temp: temp_file.into_temp_path(),
        })
    }
}

impl<T> From<T> for AtomicPath
where
    T: AsRef<OsStr>,
{
    fn from(real: T) -> Self {
        Self::new(PathBuf::from(real.as_ref()))
    }
}

impl AsRef<OsStr> for &'_ AtomicPath {
    fn as_ref(&self) -> &OsStr {
        self.real.as_os_str()
    }
}

pub struct WritableAtomicPath {
    real: PathBuf,
    temp: TempPath,
}

impl WritableAtomicPath {
    pub fn commit(self) -> Result<AtomicPath> {
        std::fs::rename(&self.temp, &self.real)?;
        Ok(AtomicPath::new(self.real))
    }
    pub fn commit_to(self, p: PathBuf) -> Result<AtomicPath> {
        std::fs::rename(&self.temp, &p)?;
        Ok(AtomicPath::new(p))
    }
}

impl AsRef<OsStr> for WritableAtomicPath {
    fn as_ref(&self) -> &OsStr {
        self.temp.as_os_str()
    }
}
