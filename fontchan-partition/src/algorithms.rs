use std::{
    borrow::Cow, collections::{HashMap, HashSet}, isize, sync::LazyLock, usize
};

use anyhow::Result;
use fontchan_unicode::{URange, URangeBuilder};
use fontchan_util::{autobox, factory, Registry};
use fontchan_util::{routine, Req};

use crate::{
    char_base::CharBaseProvider,
    char_freq::CharFreqProvider,
    pages::{Page, PagesProvider},
    PartSize,
};

fn do_partition(mut iter: impl Iterator<Item = char>, len: usize, num: usize) -> Vec<URange> {
    let n_chunks = len / num;
    let residual = len % num;
    let mut res = Vec::with_capacity(n_chunks);
    for i in 0..n_chunks {
        let mut chunk_size = num;
        if i == n_chunks - 1 {
            chunk_size += residual;
        }
        let str = iter.by_ref().take(chunk_size).collect::<String>();
        res.push(URangeBuilder::from_chars(str.chars()).build());
    }
    return res;
}

fn do_partition_exact(iter: impl ExactSizeIterator<Item = char>, num: usize) -> Vec<URange> {
    let len = iter.len();
    do_partition(iter, len, num)
}

pub(crate) struct AlgorithmContext {
    pub(crate) part_size: PartSize,
    pub(crate) char_base: Option<Box<dyn CharBaseProvider>>,
    pub(crate) char_freq: Option<Box<dyn CharFreqProvider>>,
    pub(crate) pages: Option<Box<dyn PagesProvider>>,
}

pub(crate) trait AlgorithmImpl {
    fn partition(&self, config: &AlgorithmContext) -> Vec<URange>;
}
autobox!(AlgorithmImpl);

pub struct SortByOccurrence;

impl SortByOccurrence {
    fn new(ctx: &AlgorithmContext) -> Result<Self> {
        if ctx.char_base.is_none() && ctx.char_freq.is_none() && ctx.pages.is_none() {
            return Err(anyhow::anyhow!("No input data"));
        }
        Ok(Self)
    }
    fn case_with_pages_only(num: usize, pages: Cow<[Page]>) -> Vec<URange> {
        let mut stats = HashMap::<char, isize>::new();
        for &ch in pages.iter().flatten() {
            *stats.entry(ch).or_insert(0) -= 1;
        }
        let mut chars: Vec<_> = stats.into_iter().map(|(c, f)| (f, c)).collect();
        chars.sort_unstable();
        do_partition_exact(chars.into_iter().map(|(_, c)| c), num)
    }
    fn lift_ascii<T: Copy>(stats: &mut HashMap<char, T>, value: T) {
        for char in '\u{0}'..='\u{ff}' {
            stats.entry(char).and_modify(|f| *f = value);
        }
    }
    fn case_with_pages_generic(
        num: usize,
        pages: Cow<[Page]>,
        char_freq: Option<Cow<[char]>>,
        char_base: Option<Cow<HashSet<char>>>,
    ) -> Vec<URange> {
        let char_base = char_base.as_ref();
        let freq_stats = char_freq
            .iter()
            .flat_map(Cow::as_ref)
            .filter(|c| char_base.map_or(true, |cb| cb.contains(c)))
            .cloned()
            .zip(0isize..);
        let mut stats = char_base
            .into_iter()
            .flat_map(Cow::as_ref)
            .cloned()
            .zip(std::iter::repeat(isize::MAX))
            .chain(freq_stats)
            .collect::<HashMap<_, _>>();
        for ch in pages.iter().flatten() {
            stats.entry(*ch).and_modify(|f| *f = (*f).min(0) - 1);
        }
        Self::lift_ascii(&mut stats, isize::MIN);
        let mut chars: Vec<_> = stats.into_iter().map(|(c, f)| (f, c)).collect();
        chars.sort_unstable();
        do_partition_exact(chars.into_iter().map(|(_, c)| c), num)
    }
    fn case_with_charfreq_only(num: usize, char_freq: Cow<[char]>) -> Vec<URange> {
        do_partition_exact(char_freq.into_iter().cloned(), num)
    }
    fn case_with_charfreq_charbase(
        num: usize,
        char_freq: Cow<[char]>,
        char_base: Cow<HashSet<char>>,
    ) -> Vec<URange> {
        let mut seq: Vec<_> = char_freq
            .iter()
            .filter(|c| char_base.contains(c))
            .cloned()
            .zip(0usize..)
            .chain(char_base.iter().cloned().zip(std::iter::repeat(usize::MAX)))
            .collect();
        seq.sort_unstable();
        seq.dedup_by_key(|(c, _)| *c);
        seq.sort_unstable_by_key(|(c, i)| (*i, *c));
        do_partition_exact(seq.into_iter().map(|(c, _)| c), num)
    }
    fn case_with_charbase(num: usize, char_base: Cow<HashSet<char>>) -> Vec<URange> {
        let mut seq: Vec<_> = char_base.iter().cloned().collect();
        seq.sort_unstable();
        do_partition_exact(seq.into_iter(), num)
    }
}

impl AlgorithmImpl for SortByOccurrence {
    fn partition(&self, config: &AlgorithmContext) -> Vec<URange> {
        let num = match config.part_size {
            PartSize::Chars(num) => num,
        };
        let pages = config.pages.as_ref().map(|p| p.pages());
        let char_base = config.char_base.as_ref().map(|p| p.char_base());
        let char_freq = config.char_freq.as_ref().map(|p| p.char_freq());
        match (pages, char_base, char_freq) {
            (Some(pages), None, None) => SortByOccurrence::case_with_pages_only(num, pages),
            (Some(pages), char_base, char_freq) => {
                SortByOccurrence::case_with_pages_generic(num, pages, char_freq, char_base)
            }
            (None, None, Some(char_freq)) => {
                SortByOccurrence::case_with_charfreq_only(num, char_freq)
            }
            (None, Some(char_base), Some(char_freq)) => {
                SortByOccurrence::case_with_charfreq_charbase(num, char_freq, char_base)
            }
            (None, Some(char_base), None) => SortByOccurrence::case_with_charbase(num, char_base),
            _ => unreachable!(),
        }
    }
}

pub(crate) static ALGORITHM_REGISTRY: LazyLock<Registry<AlgorithmContext, dyn AlgorithmImpl, Req>> =
    LazyLock::new(|| {
        Registry::new()
            .add(
                "sort_by_occurrence",
                factory!(SortByOccurrence::new, [context]?),
            )
            .with_default(routine!("sort_by_occurrence"))
    });
