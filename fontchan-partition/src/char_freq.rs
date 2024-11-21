use std::{borrow::Cow, sync::LazyLock};

use fontchan_util::{autobox, factory, routine, Opt, Registry};

use crate::config::Context;

pub trait CharFreqProvider {
    fn char_freq(&self) -> Cow<[char]>;
}

autobox!(CharFreqProvider);

include!("../freq-preset/freq_zh.rs");

pub struct PresetZH;

impl CharFreqProvider for PresetZH {
    fn char_freq(&self) -> Cow<[char]> {
        Cow::Borrowed(FREQ_PRESET_ZH)
    }
}

pub(crate) static CHAR_FREQ_REGISTRY: LazyLock<Registry<Context, dyn CharFreqProvider, Opt>> =
    LazyLock::new(|| {
        Registry::new()
            .add("preset_zh", factory!(PresetZH))
            .with_default(routine!("preset_zh"))
    });
