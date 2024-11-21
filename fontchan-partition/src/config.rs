use std::sync::Arc;

use fontchan_util::{Con, LazyFile, Opt, Routine};
use serde::Deserialize;

#[derive(Debug, Deserialize, Copy, Clone)]
#[serde(untagged)]
pub enum PartSize {
    Chars(usize),
}

impl Default for PartSize {
    fn default() -> Self {
        Self::Chars(200)
    }
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub part_size: PartSize,

    pub char_base: Option<Con<Routine, Opt>>,
    pub char_freq: Option<Con<Routine, Opt>>,
    pub pages: Option<Con<Routine, Opt>>,

    pub algorithm: Option<Con<Routine>>,
}

#[derive(Default)]
pub struct Context {
    pub font_files: Vec<Arc<LazyFile>>,
}
