use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Deserializer};

static WORK_DIR: OnceLock<Option<WorkDir>> = OnceLock::new();

#[derive(Clone, Debug)]
pub struct WorkDir {
    path: PathBuf,
}

impl WorkDir {
    pub fn resolve(path: &str) -> PathBuf {
        if let Some(Some(work_dir)) = WORK_DIR.get() {
            return work_dir.path.join(path);
        } else {
            panic!("WORK_DIR not initialized");
        }
    }
}

impl WorkDir {
    pub fn init_global<'a, 'de, D>(
        cli_arg: Option<OsString>,
        config_path: &'a Path,
        config_deserializer: D,
    ) -> Result<()>
    where
        D: Deserializer<'de>,
        <D as Deserializer<'de>>::Error: Sync + Send + 'static,
    {
        match Self::new(cli_arg, config_path, config_deserializer).and_then(|work_dir| {
            std::env::set_current_dir(&work_dir.path)?;
            Ok(work_dir)
        }) {
            Ok(work_dir) => {
                WORK_DIR.get_or_init(|| Some(work_dir));
                Ok(())
            }
            Err(err) => {
                WORK_DIR.get_or_init(|| None);
                Err(err)
            }
        }
    }
    fn new<'a, 'de, D>(
        cli_arg: Option<OsString>,
        config_path: &'a Path,
        config_deserializer: D,
    ) -> Result<Self>
    where
        D: Deserializer<'de>,
        <D as Deserializer<'de>>::Error: Sync + Send + 'static,
    {
        let config_path = config_path.canonicalize()?;
        let config_dir = config_path
            .parent()
            .ok_or_else(|| anyhow!("cannot resolve config dir from: {}", config_path.display()))?;
        if let Some(cli_arg) = cli_arg {
            return Self::parse(cli_arg, config_dir);
        }

        #[derive(Deserialize, Debug)]
        struct Config {
            work_dir: Option<PathBuf>,
        }

        let config = Config::deserialize(config_deserializer)?;
        if let Some(input) = config.work_dir {
            return Self::parse(input, config_dir);
        }
        Ok(Self {
            path: config_dir.to_path_buf(),
        })
    }
    fn parse(input: impl AsRef<OsStr>, config_dir: &Path) -> Result<Self> {
        let input = input.as_ref();
        let input_bytes = input.as_encoded_bytes();
        let path = if let Some(suffix) = input_bytes.strip_prefix(b"<CWD>") {
            let mut path = std::env::current_dir()?.into_os_string();
            path.push(unsafe { OsStr::from_encoded_bytes_unchecked(suffix) });
            path.into()
        } else if let Some(suffix) = input_bytes.strip_prefix(b"<CFD>") {
            let mut path = config_dir.as_os_str().to_os_string();
            path.push(unsafe { OsStr::from_encoded_bytes_unchecked(suffix) });
            path.into()
        } else {
            config_dir.join(input)
        };
        Ok(Self { path })
    }
}
