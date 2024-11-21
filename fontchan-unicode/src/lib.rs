use std::{borrow::Cow, vec};

use anyhow::{anyhow, bail, Result};
use fontchan_util::{CloneS, Hasher, UpdateInto};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct USpan {
    pub start: char,
    pub end: char,
}

impl USpan {
    pub fn size(&self) -> isize {
        self.end as isize - self.start as isize + 1
    }
    pub fn is_single(&self) -> bool {
        self.start == self.end
    }
    fn to(&self, other: USpan) -> Option<USpan> {
        self.is_single().then_some(())?;
        other.is_single().then_some(())?;
        (self.start <= other.start).then_some(())?;
        Some(USpan {
            start: self.start,
            end: other.end,
        })
    }
}

impl USpan {
    fn merge_with(&self, other: &Self) -> Option<Self> {
        let (mut lhs, mut rhs) = (self, other);
        if lhs > rhs {
            (lhs, rhs) = (rhs, lhs);
        }
        if lhs.end as u32 + 1 < rhs.start as u32 {
            return None;
        }
        Some(Self {
            start: lhs.start,
            end: rhs.end.max(lhs.end),
        })
    }
    fn chars(&self) -> impl Iterator<Item = char> + use<'_> {
        self.start..=self.end
    }
}

impl Ord for USpan {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.start
            .cmp(&other.start)
            .then_with(|| self.end.cmp(&other.end))
    }
}

impl PartialOrd for USpan {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct URangeBuilder {
    spans: Vec<USpan>,
}

impl URangeBuilder {
    pub fn new() -> Self {
        Self { spans: vec![] }
    }

    pub fn from_chars(chars: impl Iterator<Item = char>) -> Self {
        Self {
            spans: chars.map(|ch| USpan { start: ch, end: ch }).collect(),
        }
    }

    pub fn push(&mut self, range: USpan) -> &mut Self {
        self.spans.push(range);
        self
    }

    pub fn build(self) -> URange {
        let mut spans = self.spans;
        spans.sort();
        let mut new_span = Vec::<USpan>::with_capacity(spans.len());
        for range in spans {
            if let Some(last) = new_span.last_mut() {
                if let Some(merged) = last.merge_with(&range) {
                    *last = merged;
                    continue;
                }
            }
            new_span.push(range);
        }
        new_span.sort_by_key(|r| (!r.is_single(), r.start));
        let num_single = new_span.iter().take_while(|s| s.is_single()).count();
        URange {
            spans: new_span,
            num_single,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct URange {
    spans: Vec<USpan>,
    num_single: usize,
}

impl URange {
    pub fn as_chars(&self) -> impl Iterator<Item = char> + use<'_> {
        self.spans.iter().flat_map(|range| range.chars())
    }
    pub fn single_count(&self) -> usize {
        self.num_single
    }
    pub fn multi_count(&self) -> usize {
        self.spans.len() - self.num_single
    }
}

impl UpdateInto for &'_ URange {
    fn update_into(&self, hasher: &mut dyn Hasher) {
        for range in &self.spans {
            hasher.update(&(range.start as u32).to_le_bytes());
            hasher.update(&(range.end as u32).to_le_bytes());
        }
    }
}

impl AsRef<[USpan]> for URange {
    fn as_ref(&self) -> &[USpan] {
        &self.spans
    }
}

impl URangeBuilder {
    pub fn from_css_syntax(input: impl AsRef<str>) -> Result<URangeBuilder> {
        fn parse_part(mut part: &str) -> Option<USpan> {
            part = part
                .trim()
                .strip_prefix(&['u', 'U'])
                .and_then(|part| part.strip_prefix('+'))
                .unwrap_or(part);
            let n_wc = {
                let orig_len = part.len();
                part = part.trim_end_matches('?');
                orig_len - part.len()
            };
            let num = part
                .is_empty()
                .then_some(0)
                .or_else(|| u32::from_str_radix(part, 16).ok())?;
            let (start, end) = if n_wc > 0 {
                let shift = 4 * n_wc as u32;
                (
                    num.checked_shl(shift)?,
                    num.checked_add(1)?.checked_shl(shift)?.checked_sub(1)?,
                )
            } else if part.is_empty() {
                return None;
            } else {
                (num, num)
            };
            let start = char::from_u32(start)?;
            let end = char::from_u32(end)?;
            (start <= end).then_some(USpan { start, end })
        }
        let mut set = Vec::<USpan>::new();
        for piece in input.as_ref().split(',') {
            let make_error = || anyhow!("{:?}: invalid syntax", piece);
            let mut parts = piece.trim().split('-');
            let start = parts.next().and_then(parse_part).ok_or_else(make_error)?;
            let range = match parts.next() {
                Some(end) => {
                    let end = parse_part(end).ok_or_else(make_error)?;
                    start.to(end).ok_or_else(make_error)?
                }
                None => start,
            };
            if parts.next().is_some() {
                bail!("{:?}: too many '-'", piece);
            }
            set.push(range);
        }
        Ok(URangeBuilder { spans: set })
    }
}

#[test]
fn test_unicode_range_from_css_syntax() {
    let set = URangeBuilder::from_css_syntax("U+1F600-1F64F, U+1F680-u+1F6C5, U+???, U+4, U+3??")
        .unwrap();
    assert_eq!(
        &set,
        &URangeBuilder {
            spans: vec![
                USpan {
                    start: '\u{1f600}',
                    end: '\u{1f64f}'
                },
                USpan {
                    start: '\u{1f680}',
                    end: '\u{1f6c5}'
                },
                USpan {
                    start: '\u{0}',
                    end: '\u{fff}'
                },
                USpan {
                    start: '\u{4}',
                    end: '\u{4}'
                },
                USpan {
                    start: '\u{300}',
                    end: '\u{3ff}'
                },
            ],
        }
    );
    dbg!(set.build());
}

#[derive(Debug)]
pub struct UName<'a>(Cow<'a, str>);

impl<'a, 'b: 'a> CloneS<'a, 'b> for UName<'a> {
    type Output = UName<'b>;

    fn clone_s(&'b self) -> Self::Output {
        UName(self.0.clone_s())
    }
}

impl<'a, T> From<T> for UName<'a>
where
    T: Into<Cow<'a, str>>,
{
    fn from(name: T) -> Self {
        Self(name.into())
    }
}

impl<'a, 'b: 'a> Into<Cow<'b, str>> for &'b UName<'a> {
    fn into(self) -> Cow<'b, str> {
        Cow::Borrowed(&*self.0)
    }
}

impl AsRef<str> for UName<'_> {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

pub struct UEntry<'a> {
    pub name: UName<'a>,
    pub range: &'a URange,
}

impl<'a, 'b: 'a> CloneS<'a, 'b> for UEntry<'a> {
    type Output = UEntry<'b>;
    fn clone_s(&'b self) -> Self::Output {
        Self {
            name: self.name.clone_s(),
            range: self.range,
        }
    }
}
