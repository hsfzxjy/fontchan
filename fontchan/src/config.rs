use std::{path::PathBuf, sync::Arc};

use fontchan_util::{Con, Routine};
use serde::Deserialize;

use fontchan_util::LazyFile;

use crate::builder::FontOutputTmpl;

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[allow(unused)]
    work_dir: Option<std::path::PathBuf>,

    pub fonts: Vec<FontConfig>,
    pub builder: BuilderConfig,
    pub partition: fontchan_partition::Config,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct FontConfig {
    pub css: crate::builder::CSSFragments<'static>,
    pub input_path: Arc<LazyFile>,
    pub output_tmpl: Arc<FontOutputTmpl>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct BuilderConfig {
    pub js: JsBuilderConfig,
    #[serde(default)]
    pub font: FontBuilderConfig,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct JsBuilderConfig {
    pub output_path: PathBuf,
}

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct FontBuilderConfig {
    pub backend: Option<Con<Routine>>,
}
