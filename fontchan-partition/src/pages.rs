use std::borrow::Cow;
use std::cell::Cell;
use std::collections::HashSet;
use std::sync::{LazyLock, OnceLock};

use anyhow::Result;
use fontchan_util::{autobox, factory, Opt, Registry, RoutineArg};

use crate::config::Context;

#[derive(Debug, Clone)]
pub struct Page(HashSet<char>);

impl FromIterator<char> for Page {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<'a> IntoIterator for &'a Page {
    type Item = &'a char;
    type IntoIter = std::collections::hash_set::Iter<'a, char>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

pub trait PagesProvider {
    fn pages(&self) -> Cow<[Page]>;
}
autobox!(PagesProvider);

pub struct GlobPagesProvider {
    cache: OnceLock<Vec<Page>>,
    glob: Cell<Option<glob::Paths>>,
}

impl GlobPagesProvider {
    fn new(pattern: &RoutineArg) -> Result<Self> {
        let pattern = pattern.required()?;
        Ok(Self {
            cache: OnceLock::new(),
            glob: Cell::new(Some(glob::glob(&pattern)?)),
        })
    }
}

impl PagesProvider for GlobPagesProvider {
    fn pages(&self) -> Cow<[Page]> {
        use rayon::prelude::*;
        let pages = self.cache.get_or_init(|| {
            self.glob
                .take()
                .unwrap()
                .par_bridge()
                .filter_map(|path| {
                    let path = path.ok()?;
                    let content = std::fs::read_to_string(&path).ok()?;
                    Some(content.chars().collect::<Page>())
                })
                .collect::<Vec<_>>()
        });
        Cow::Borrowed(pages)
    }
}

pub(crate) static PAGES_REGISTRY: LazyLock<Registry<Context, dyn PagesProvider, Opt>> =
    LazyLock::new(|| Registry::new().add("glob", factory!(GlobPagesProvider::new, [arg]?)));
