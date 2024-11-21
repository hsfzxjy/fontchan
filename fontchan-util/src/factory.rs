use std::collections::BTreeMap;

use crate::{Con, Quant, Routine, RoutineArg};
use anyhow::{anyhow, bail, Result};

type BuildFn<C, T> = Box<dyn Sync + Send + for<'a> Fn(&'a C, &'a RoutineArg) -> Result<Box<T>>>;
struct Builder<C, T: ?Sized>(BuildFn<C, T>);

impl<F, C, T: ?Sized> From<F> for Builder<C, T>
where
    F: Sync + Send + (for<'a> Fn(&'a C, &'a RoutineArg) -> Result<Box<T>>) + 'static,
{
    fn from(f: F) -> Self {
        Self(Box::new(f))
    }
}

pub trait CoerceBoxed<T: ?Sized> {
    fn coerce_boxed(self: Box<Self>) -> Box<T>;
}

#[macro_export]
macro_rules! autobox {
    ($trait_:path) => {
        impl<T: $trait_ + 'static> From<T> for Box<dyn $trait_> {
            fn from(value: T) -> Self {
                Box::new(value)
            }
        }
    };
}

#[macro_export]
macro_rules! factory {
    ($builder:path) => {
        Box::new(|context: &'_ _, arg: &'_ _| Ok($builder.into()))
    };
    ($builder:path , [ $($arg:tt),* ] $($qmark:tt)?) => {
        factory!(@param var $builder [ $($arg),* ] $($qmark)?)
    };
    (@param $var:ident $builder:path [ $($arg:tt),* ] $($qmark:tt)?) => {
        (Box::new(|context:&'_ _, arg:&'_ _| {
            let $var = (context, arg);
            Ok($builder( $( factory!(@param $var $arg) ),* ) $($qmark)? .into())
    }   ))
    };
    (@param $var:ident context) => {$var.0};
    (@param $var:ident arg) => {$var.1};
}

pub struct Registry<C, T: ?Sized, Q: Quant> {
    table: BTreeMap<&'static str, Builder<C, T>>,
    default: Option<Con<Routine, Q>>,
}

impl<C: 'static, T: ?Sized, Q: Quant> Registry<C, T, Q> {
    pub fn new() -> Self {
        Self {
            table: BTreeMap::new(),
            default: None,
        }
    }

    #[allow(private_bounds)]
    pub fn add(mut self, name: &'static str, builder: impl Into<Builder<C, T>>) -> Self {
        self.table.insert(name, builder.into());
        self
    }

    pub fn with_default(mut self, default: Routine) -> Self {
        self.default = Some(Con::wrap(default));
        self
    }

    fn build_one(&self, context: &C, routine: &Routine) -> Result<Box<T>> {
        let Some(builder) = self.table.get(routine.name.as_ref()) else {
            bail!("routine not found: {}", routine);
        };
        builder.0(context, &routine.arg).map_err(|e| anyhow!("{}: {}", routine, e))
    }

    pub fn build(&self, context: &C, routine: &Option<Con<Routine, Q>>) -> Result<Con<Box<T>, Q>> {
        let routine = routine.as_ref();
        let Some(r) = routine.or(self.default.as_ref()) else {
            return Con::missing().ok_or_else(|| anyhow!("routine required"));
        };
        r.map_ref(|r| self.build_one(context, r)).collect_result()
    }
}
