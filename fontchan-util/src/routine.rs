use core::str;
use std::{
    borrow::Cow,
    fmt::{Debug, Write},
    ops::Deref,
};

use crate::hkt::*;
use crate::quant::*;
use anyhow::{anyhow, Result};
use serde::Deserialize;

type CowStr = Cow<'static, str>;

#[derive(Debug)]
pub struct Routine {
    pub name: CowStr,
    pub arg: RoutineArg,
}

impl Routine {
    fn parse_str(input: &str) -> Option<(&str, Option<&str>)> {
        const BRACKETS: &[char] = &['[', ']'];
        let mut splitted = input.split_inclusive(BRACKETS);
        let name = splitted.next().map(str::as_bytes);
        let arg = splitted.next().map(str::as_bytes);
        if splitted.next().is_some() {
            return None;
        }
        const fn cast(b: &[u8]) -> &str {
            unsafe { str::from_utf8_unchecked(b) }
        }
        match (name, arg) {
            (Some([.., b'[' | b']']), None) => None,
            (Some(name), None) => Some((cast(name), None)),
            (Some([name @ .., b'[']), Some([arg @ .., b']'])) if !name.is_empty() => {
                Some((cast(name), Some(cast(arg))))
            }
            _ => None,
        }
    }
    pub fn new<Q: Quant>(input: impl Into<CowStr>) -> Result<QData<Self, Q>, RoutineParseError> {
        use RoutineParseError::*;
        let input = input.into();
        if input.is_empty() {
            return Q::M::missing().ok_or(Required);
        }
        let routine = match input {
            Cow::Borrowed(input) => {
                let (name, arg) = Self::parse_str(input)
                    .ok_or(Cow::Borrowed(input))
                    .map_err(Invalid)?;
                Self {
                    name: name.into(),
                    arg: RoutineArg(arg.map(Into::into)),
                }
            }
            Cow::Owned(input) => {
                let Some((name, arg)) = Self::parse_str(&input) else {
                    return Err(Invalid(input.into()));
                };
                Self {
                    name: name.to_owned().into(),
                    arg: RoutineArg(arg.map(Into::into).map(Cow::Owned)),
                }
            }
        };
        Ok(Q::M::wrap(routine))
    }
}

impl std::fmt::Display for Routine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)?;
        if let Some(arg) = self.arg.as_ref() {
            f.write_char('[')?;
            f.write_str(arg)?;
            f.write_char(']')?;
        }
        Ok(())
    }
}

impl<'de, Q: Quant> Deserialize<'de> for Con<Routine, Q> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let input = CowStr::deserialize(deserializer)?;
        Routine::new::<Q>(input)
            .map(Con)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug)]
pub struct RoutineArg(Option<CowStr>);

impl Deref for RoutineArg {
    type Target = Option<CowStr>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RoutineArg {
    pub fn required(&self) -> Result<&str> {
        self.0
            .as_ref()
            .map(Cow::as_ref)
            .ok_or_else(|| anyhow!("Argument required"))
    }
}

#[derive(Debug)]
pub enum RoutineParseError {
    Required,
    Invalid(CowStr),
}

impl std::fmt::Display for RoutineParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use RoutineParseError::*;
        match self {
            Required => write!(f, "routine required"),
            Invalid(input) => write!(f, "invalid routine: {}", input),
        }
    }
}

#[macro_export]
macro_rules! routine {
    ($input:expr) => {
        $crate::Routine::new::<$crate::Req>($input).unwrap()
    };
}

#[test]
fn test_routine() {
    let _r = routine!("test[");
    dbg!(_r);
}
