use std::{fmt::Debug, marker::PhantomData};

use serde::Deserialize;

use crate::hkt::*;

pub type QData<T, Q> = <<Q as Quant>::M as UnaryTy>::Out<T>;

pub trait Quant {
    type M: Functor + FunctorResultExt;
}

pub struct Opt;

impl Quant for Opt {
    type M = OptionTy;
}

pub struct Req;

impl Quant for Req {
    type M = IdTy;
}

pub struct Multi;

impl Quant for Multi {
    type M = VecTy;
}

pub struct Con<T, Q: Quant = Req>(pub(crate) QData<T, Q>);

impl<Q: Quant, T> Con<T, Q> {
    pub fn wrap(data: T) -> Self {
        Con(Q::M::wrap(data))
    }
    pub fn into_data(self) -> QData<T, Q> {
        self.0
    }
    pub fn missing() -> Option<Self> {
        Q::M::missing().map(Con)
    }
}

impl<Q: Quant, T, E> Con<Result<T, E>, Q> {
    pub fn transpose(self) -> Result<Con<T, Q>, E> {
        Q::M::transpose(self.0).map(Con)
    }
}

impl<T, Q: Quant> From<T> for Con<T, Q> {
    fn from(value: T) -> Self {
        Self::wrap(value)
    }
}

impl<T, Q: Quant> Debug for Con<T, Q>
where
    QData<T, Q>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Con").field(&self.0).finish()
    }
}

impl<'de, Q: Quant, T> Deserialize<'de> for Con<T, Q>
where
    T: for<'de2> Deserialize<'de2>,
    QData<T, Q>: for<'de2> Deserialize<'de2>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        QData::<T, Q>::deserialize(deserializer).map(Con)
    }
}

impl<T, Q: Quant> Con<T, Q> {
    pub fn map<U, F>(self, f: F) -> ConMap<T, Q, U, F>
    where
        F: FnMut(T) -> U,
    {
        ConMap {
            f,
            source: self.0,
            _q: PhantomData,
        }
    }
    pub fn map_ref<'a, U, F>(&'a self, f: F) -> ConMapRef<'a, T, Q, U, F>
    where
        F: FnMut(&'a T) -> U,
    {
        ConMapRef {
            f,
            source: &self.0,
            _q: PhantomData,
        }
    }
}

pub struct ConMapRef<'a, T, Q: Quant, U, F> {
    f: F,
    source: &'a QData<T, Q>,
    _q: PhantomData<(Q, dyn FnMut(&'a T) -> U)>,
}

impl<'a, T, Q: Quant, U, F> ConMapRef<'a, T, Q, U, F>
where
    F: FnMut(&'a T) -> U,
{
    pub fn map<V, F2>(self, mut f2: F2) -> ConMapRef<'a, T, Q, V, impl FnMut(&'a T) -> V>
    where
        F2: FnMut(U) -> V + 'static,
    {
        let ConMapRef { mut f, source, .. } = self;
        ConMapRef {
            f: move |x| f2((f)(x)),
            source,
            _q: PhantomData,
        }
    }
    pub fn collect(self) -> Con<U, Q> {
        Con(Q::M::map_ref(&self.source, self.f))
    }
}

impl<'a, T, Q: Quant, U, F, E> ConMapRef<'a, T, Q, Result<U, E>, F>
where
    F: FnMut(&'a T) -> Result<U, E>,
{
    pub fn collect_result(self) -> Result<Con<U, Q>, E> {
        Q::M::map_ref_transpose(&self.source, self.f).map(Con)
    }
}

pub struct ConMap<T, Q: Quant, U, F> {
    f: F,
    source: QData<T, Q>,
    _q: PhantomData<(Q, dyn FnMut(T) -> U)>,
}

impl<T, Q: Quant, U, F> ConMap<T, Q, U, F>
where
    F: FnMut(T) -> U,
{
    pub fn map<V, F2>(self, mut f2: F2) -> ConMap<T, Q, V, impl FnMut(T) -> V>
    where
        F2: FnMut(U) -> V + 'static,
    {
        let ConMap { mut f, source, .. } = self;
        ConMap {
            f: move |x| f2((f)(x)),
            source,
            _q: PhantomData,
        }
    }
    pub fn collect(self) -> Con<U, Q> {
        Con(Q::M::map(self.source, self.f))
    }
}

impl<T, Q: Quant, U, F, E> ConMap<T, Q, U, F>
where
    F: FnMut(T) -> Result<U, E>,
{
    pub fn collect_result(self) -> Result<Con<U, Q>, E> {
        Q::M::map_transpose(self.source, self.f).map(Con)
    }
}
