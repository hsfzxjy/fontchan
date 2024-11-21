use std::{
    borrow::Cow,
    collections::HashSet,
    sync::{Arc, LazyLock, OnceLock},
};

use allsorts::{
    binary::read::ReadScope,
    font_data::FontData,
    tables::{cmap::CmapSubtable, FontTableProvider},
    Font,
};
use anyhow::{bail, Result};
use fontchan_util::{autobox, factory, routine, LazyFile, Opt, Registry};

use crate::config::Context;

pub trait CharBaseProvider {
    fn char_base(&self) -> Cow<HashSet<char>>;
}
autobox!(CharBaseProvider);

struct FromFonts {
    fonts: Vec<Arc<LazyFile>>,
    cache: OnceLock<HashSet<char>>,
}

impl CharBaseProvider for FromFonts {
    fn char_base(&self) -> Cow<HashSet<char>> {
        let chars = self.cache.get_or_init(|| {
            let mut chars = HashSet::new();
            for font in &self.fonts {
                let buffer = font.content().unwrap();
                let scope = ReadScope::new(&buffer);
                let font_file = scope.read::<FontData>().unwrap();
                let table_provider = font_file.table_provider(0).unwrap();
                let mut font = allsorts::font::Font::new(Box::new(table_provider)).unwrap();
                dump_cmap(&mut font, &mut chars).unwrap();
            }
            chars
        });
        Cow::Borrowed(chars)
    }
}

impl FromFonts {
    pub fn new(context: &Context) -> Self {
        Self {
            fonts: context.font_files.clone(),
            cache: OnceLock::new(),
        }
    }
}

fn dump_cmap<T: FontTableProvider>(font: &mut Font<T>, chars: &mut HashSet<char>) -> Result<()> {
    let cmap_subtable = ReadScope::new(font.cmap_subtable_data()).read::<CmapSubtable<'_>>()?;
    let encoding = font.cmap_subtable_encoding;
    use allsorts::font::Encoding;

    if encoding != Encoding::Unicode {
        bail!("Unsupported encoding: {:?}", encoding);
    }

    cmap_subtable.mappings_fn(|ch, _| {
        chars.insert(ch.try_into().unwrap());
    })?;

    Ok(())
}

pub(crate) static CHAR_BASE_REGISTRY: LazyLock<Registry<Context, dyn CharBaseProvider, Opt>> =
    LazyLock::new(|| {
        Registry::new()
            .add("from_fonts", factory!(FromFonts::new, [context]))
            .with_default(routine!("from_fonts"))
    });
