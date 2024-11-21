pub trait UnaryTy {
    type Out<T>;
}

pub trait Functor: UnaryTy {
    fn wrap<T>(value: T) -> Self::Out<T>;
    fn missing<T>() -> Option<Self::Out<T>> {
        None
    }
    fn map<T, U, F>(value: Self::Out<T>, f: F) -> Self::Out<U>
    where
        F: FnMut(T) -> U;
    fn map_ref<'a, T: 'a, U, F>(value: &'a Self::Out<T>, f: F) -> Self::Out<U>
    where
        F: FnMut(&'a T) -> U;
}

pub struct VecTy;

impl UnaryTy for VecTy {
    type Out<T> = Vec<T>;
}

impl Functor for VecTy {
    fn wrap<T>(value: T) -> Self::Out<T> {
        vec![value]
    }

    fn map<T, U, F>(value: Self::Out<T>, f: F) -> Self::Out<U>
    where
        F: FnMut(T) -> U,
    {
        value.into_iter().map(f).collect()
    }

    fn map_ref<'a, T: 'a, U, F>(value: &'a Self::Out<T>, f: F) -> Self::Out<U>
    where
        F: FnMut(&'a T) -> U,
    {
        value.iter().map(f).collect()
    }
}

pub struct OptionTy;

impl UnaryTy for OptionTy {
    type Out<T> = Option<T>;
}

impl Functor for OptionTy {
    fn wrap<T>(value: T) -> Self::Out<T> {
        Some(value)
    }
    fn missing<T>() -> Option<Self::Out<T>> {
        Some(None)
    }
    fn map<T, U, F>(value: Self::Out<T>, f: F) -> Self::Out<U>
    where
        F: FnMut(T) -> U,
    {
        value.map(f)
    }
    fn map_ref<'a, T: 'a, U, F>(value: &'a Self::Out<T>, f: F) -> Self::Out<U>
    where
        F: FnMut(&'a T) -> U,
    {
        value.as_ref().map(f)
    }
}

pub struct IdTy;

impl UnaryTy for IdTy {
    type Out<T> = T;
}

impl Functor for IdTy {
    fn wrap<T>(value: T) -> Self::Out<T> {
        value
    }
    fn map<T, U, F>(value: Self::Out<T>, mut f: F) -> Self::Out<U>
    where
        F: FnMut(T) -> U,
    {
        f(value)
    }
    fn map_ref<'a, T: 'a, U, F>(value: &'a Self::Out<T>, mut f: F) -> Self::Out<U>
    where
        F: FnMut(&'a T) -> U,
    {
        f(value)
    }
}

pub trait FunctorResultExt: UnaryTy {
    fn transpose<E, T>(v: Self::Out<Result<T, E>>) -> Result<Self::Out<T>, E>;
    fn map_transpose<E, T, F, U>(v: Self::Out<T>, f: F) -> Result<Self::Out<U>, E>
    where
        F: FnMut(T) -> Result<U, E>;
    fn map_ref_transpose<'a, E, T: 'a, U, F>(v: &'a Self::Out<T>, f: F) -> Result<Self::Out<U>, E>
    where
        F: FnMut(&'a T) -> Result<U, E>;
}

impl FunctorResultExt for IdTy {
    fn transpose<E, T>(v: Result<T, E>) -> Result<T, E> {
        v
    }
    fn map_transpose<E, T, F, U>(v: T, mut f: F) -> Result<U, E>
    where
        F: FnMut(T) -> Result<U, E>,
    {
        f(v)
    }
    fn map_ref_transpose<'a, E, T: 'a, U, F>(v: &'a T, mut f: F) -> Result<U, E>
    where
        F: FnMut(&'a T) -> Result<U, E>,
    {
        f(v)
    }
}

impl FunctorResultExt for OptionTy {
    fn transpose<E, T>(v: Option<Result<T, E>>) -> Result<Option<T>, E> {
        v.transpose()
    }
    fn map_transpose<E, T, F, U>(v: Option<T>, f: F) -> Result<Option<U>, E>
    where
        F: FnMut(T) -> Result<U, E>,
    {
        v.map(f).transpose()
    }
    fn map_ref_transpose<'a, E, T: 'a, U, F>(v: &'a Self::Out<T>, f: F) -> Result<Self::Out<U>, E>
    where
        F: FnMut(&'a T) -> Result<U, E>,
    {
        v.as_ref().map(f).transpose()
    }
}

impl FunctorResultExt for VecTy {
    fn transpose<E, T>(v: Vec<Result<T, E>>) -> Result<Vec<T>, E> {
        v.into_iter().collect()
    }

    fn map_transpose<E, T, F, U>(v: Self::Out<T>, f: F) -> Result<Self::Out<U>, E>
    where
        F: FnMut(T) -> Result<U, E>,
    {
        v.into_iter().map(f).collect()
    }

    fn map_ref_transpose<'a, E, T: 'a, U, F>(v: &'a Self::Out<T>, f: F) -> Result<Self::Out<U>, E>
    where
        F: FnMut(&'a T) -> Result<U, E>,
    {
        v.iter().map(f).collect()
    }
}
