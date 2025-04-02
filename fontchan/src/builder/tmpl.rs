use std::{
    borrow::Cow,
    fmt::Debug,
    marker::PhantomData,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};

#[derive(Debug)]
pub struct PathTmpl<'a, P: TmplParams> {
    pub directory: Cow<'a, Path>,
    pub name_template: Tmpl<'a, P>,
    _params: PhantomData<P>,
}

impl<'a, 'de, P> serde::Deserialize<'de> for PathTmpl<'a, P>
where
    P: TmplParams,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let input = Cow::deserialize(deserializer)?;
        Self::new(input).map_err(serde::de::Error::custom)
    }
}

impl<'a, P> PathTmpl<'a, P>
where
    P: TmplParams,
{
    pub fn new(input: Cow<'a, str>) -> Result<Self> {
        let p = PathBuf::from(input.as_ref());
        let Some(mut parent) = p.parent() else {
            bail!("Parent directory not found for path: {:?}", p);
        };
        if parent == Path::new("") {
            parent = Path::new(".");
        }
        let parent = parent.to_owned();
        let Some(file_name) = p.file_name() else {
            bail!("File name not found for path: {:?}", p);
        };
        let file_name = file_name.to_string_lossy();
        {
            let parent = parent.to_string_lossy();
            for mvar in P::METAVARS {
                if parent.contains(mvar) {
                    bail!("Parent directory contains a metavariable: {:?}", mvar);
                }
            }
        }
        for mvar in P::METAVARS {
            if file_name.match_indices(mvar).count() != 1 {
                bail!(
                    "File name must contain exactly one instance of metavariable: {:?}",
                    mvar
                );
            }
        }
        Ok(Self {
            directory: Cow::Owned(parent),
            name_template: Tmpl::new(Cow::Owned(file_name.into_owned())),
            _params: PhantomData,
        })
    }
    pub fn render(&self, params: &P) -> Cow<Path> {
        let file_name = self.name_template.render(params);
        self.directory.join(Path::new(file_name.as_ref())).into()
    }
}

#[derive(Debug, Clone)]
pub struct Tmpl<'a, P> {
    template: Cow<'a, str>,
    _params: PhantomData<P>,
}

impl<'a, P> Tmpl<'a, P> {
    pub const fn new(template: Cow<'a, str>) -> Self {
        Self {
            template,
            _params: PhantomData,
        }
    }
    pub const fn new_str(template: &'a str) -> Self {
        Self {
            template: Cow::Borrowed(template),
            _params: PhantomData,
        }
    }
    pub fn as_str(&self) -> &str {
        &*self.template
    }
}

impl<'a, P> Tmpl<'a, P>
where
    P: TmplParams,
{
    pub fn render(&self, params: &P) -> Cow<str> {
        let mut result = match &self.template {
            Cow::Borrowed(s) => Cow::Borrowed(*s),
            Cow::Owned(s) => Cow::Borrowed(s.as_str()),
        };
        for (i, param) in P::METAVARS.iter().zip(params.as_params()) {
            result = Cow::Owned(result.replace(i, param));
        }
        result
    }
}

pub trait TmplParams: Debug {
    const METAVARS: &'static [&'static str];
    fn as_params(&self) -> impl IntoIterator<Item = &str>;
}

#[macro_export]
macro_rules! paramdef {
    ($name:ident, $builder:ident, $( $field:ident = $mvars:literal ),+ ) => {
        paramdef!(, $name, $builder, $( $field = $mvars ),+ );
    };
    ($vis:vis, $name:ident, $builder:ident, $( $field:ident = $mvars:literal ),+ ) => {
        #[derive(Debug, Clone)]
        $vis struct $name<'a> {
            $($field: std::borrow::Cow<'a, str>,)+
        }

        impl<'a> $crate::builder::tmpl::TmplParams for $name<'a> {
            const METAVARS: &'static [&'static str] = &[$($mvars),+];
            fn as_params(&self) -> impl IntoIterator<Item = &str> {
                [$(self.$field.as_ref()),+]
            }
        }

        paramdef!(@BUILDER $name $builder $);
    };
    (@BUILDER $name:ident $builder:ident $d:tt) => {
        macro_rules! $builder {
            ($d ($d f: ident = $d v:expr),*) => {
                $name {
                    $d ($d f: $d v.into(),)*
                }
            };
        }
    };
}
