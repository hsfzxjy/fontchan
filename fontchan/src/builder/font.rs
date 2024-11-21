use std::{
    borrow::Cow,
    collections::HashMap,
    ffi::{OsStr, OsString},
    fs,
    os::windows::fs::FileTypeExt,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
};

use anyhow::{anyhow, bail, Result};
use fontchan_unicode::{UEntry, UName};
use rayon::prelude::*;

use super::tmpl::{PathTmpl, Tmpl, TmplParams};
use crate::{
    config::{Config, FontConfig},
    paramdef,
};
use fontchan_util::{
    autobox, exts::*, factory, routine, AtomicPath, DigestString, Digester, Hasher, LazyFile,
    Registry, Req, UpdateInto, WritableAtomicPath,
};

pub type FontOutputTmpl = PathTmpl<'static, FontOutputTmplParams<'static>>;
paramdef!(pub, FontOutputTmplParams, font_out_params, fid = "<FID>");

pub trait FontProvider: Sync {
    fn file(&self) -> &LazyFile;
}

impl FontProvider for Arc<LazyFile> {
    fn file(&self) -> &LazyFile {
        &self
    }
}

struct TmpFileGuard<'a>(#[allow(unused)] &'a mut DestInfo, WritableAtomicPath);

impl TmpFileGuard<'_> {
    fn commit(self) -> Result<()> {
        self.1.commit()?;
        Ok(())
    }
}

impl TmpFileGuard<'_> {
    fn path(&self) -> &OsStr {
        self.1.as_ref()
    }
}

#[derive(Debug)]
struct DestInfo {
    tmpl: Arc<FontOutputTmpl>,
    fid: Fid<'static>,
    digest: DigestString,
    file_path: PathBuf,
}

impl DestInfo {
    fn new<'a>(tmpl: Arc<FontOutputTmpl>, name: UName<'a>, digest: DigestString) -> Self {
        let fid = Fid::new(&name, &digest);
        let file_path = tmpl
            .render(&font_out_params!(fid = fid.as_str()))
            .into_owned();
        Self {
            tmpl,
            digest,
            fid,
            file_path,
        }
    }
    fn as_writable(&mut self) -> Result<TmpFileGuard> {
        let tmp = AtomicPath::from(&self.file_path).into_writable()?;
        Ok(TmpFileGuard(self, tmp))
    }
    fn changed(&self) -> Result<bool> {
        Ok(!std::fs::exists(&self.file_path)?)
    }
}

pub struct Context {
    pub source: Box<dyn FontProvider>,
    pub dest_tmpl: Arc<FontOutputTmpl>,
}

impl Context {
    fn get_hash<'a>(&self, backend: &dyn Backend, entry: &UEntry<'a>) -> DigestString {
        Digester::new()
            .push(self.source.file())
            .push(entry.range)
            .push_dyn(backend.characteristics())
            .base64_result()
    }
    fn dest_info<'a>(&self, backend: &dyn Backend, entry: &UEntry<'a>) -> DestInfo {
        let digest = self.get_hash(backend, entry);
        DestInfo::new(self.dest_tmpl.clone(), entry.name.clone_s(), digest)
    }
}

impl From<&FontConfig> for Context {
    fn from(spec: &FontConfig) -> Self {
        Self {
            source: Box::new(spec.input_path.clone()),
            dest_tmpl: spec.output_tmpl.clone(),
        }
    }
}

pub struct Builder {
    contexts: Vec<Context>,
    backend: Box<dyn Backend>,
}

impl Builder {
    pub fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            contexts: config.fonts.iter().map(Into::into).collect(),
            backend: BACKEND_REGISTRY
                .build(&(), &config.builder.font.backend)?
                .into_data(),
        })
    }

    pub fn build<'a>(
        &self,
        entries: impl IntoParallelIterator<Item = UEntry<'a>>,
    ) -> Result<BuildResults<'a>> {
        let contexts = &self.contexts;
        let his = History::new(contexts)?;
        let history = &his;
        entries
            .into_par_iter()
            .flat_map(|entry| {
                let entry = Arc::new(entry);
                contexts.par_iter().map(move |ctx| {
                    let mut dest_info = ctx.dest_info(&*self.backend, &entry);
                    if let Some(old_files) = history.query(&dest_info) {
                        for old in old_files {
                            if old.as_path() != dest_info.file_path {
                                let _ = std::fs::remove_file(&old)?;
                            }
                        }
                    }
                    if dest_info.changed()? {
                        self.backend
                            .do_subset(&ctx, &mut dest_info, entry.clone())?;
                    }
                    Ok(BuildResult {
                        fid: dest_info.fid,
                        digest: dest_info.digest,
                    })
                })
            })
            .collect::<Result<Vec<_>>>()
            .map(BuildResults::new_with(contexts.len()))
    }
}

#[derive(Debug)]
pub struct Fid<'a>(Cow<'a, str>);

impl<'a> Fid<'a> {
    fn new(name: &UName<'_>, digest: &'_ DigestString) -> Fid<'static> {
        Fid(Cow::Owned([name.as_ref(), "_", &digest[..8]].concat()))
    }
    pub fn as_str(&self) -> &str {
        &*self.0
    }
}

pub struct BuildResult<'a> {
    pub fid: Fid<'a>,
    #[allow(unused)]
    pub digest: DigestString,
}

/// Results are arranged as entry-major, context-minor order, i.e.,
/// [entry1_ctx1, entry1_ctx2, ..., entry2_ctx1, entry2_ctx2, ...]
pub struct BuildResults<'a> {
    ctx_count: usize,
    results: Vec<BuildResult<'a>>,
}

impl<'a> BuildResults<'a> {
    fn new_with(ctx_count: usize) -> impl FnOnce(Vec<BuildResult<'a>>) -> Self {
        move |results| Self { ctx_count, results }
    }
    pub fn entry_minor_iter(&self) -> impl Iterator<Item = &BuildResult<'a>> {
        let entry_count = self.results.len() / self.ctx_count;
        (0..self.ctx_count).flat_map(move |i| {
            self.results
                .iter()
                .skip(i)
                .step_by(self.ctx_count)
                .take(entry_count)
        })
    }
}

static BACKEND_REGISTRY: LazyLock<Registry<(), dyn Backend, Req>> = LazyLock::new(|| {
    Registry::new()
        .add("pyft", factory!(PyftBackend))
        .with_default(routine!("pyft"))
});

trait Backend: Sync {
    fn do_subset<'a>(
        &self,
        ctx: &Context,
        dest_info: &mut DestInfo,
        entry: Arc<UEntry<'a>>,
    ) -> Result<()>;
    fn characteristics(&self) -> &dyn UpdateInto;
}
autobox!(Backend);

pub struct PyftBackend;

impl UpdateInto for PyftBackend {
    fn update_into(&self, hasher: &mut dyn Hasher) {
        hasher.update(b"pyftsubset");
        hasher.update(b"--ignore-missing-glyphs");
    }
}

impl Backend for PyftBackend {
    fn characteristics(&self) -> &dyn UpdateInto {
        self
    }
    fn do_subset<'a>(
        &self,
        ctx: &Context,
        dest_info: &mut DestInfo,
        entry: Arc<UEntry<'a>>,
    ) -> Result<()> {
        use std::io::Write;
        let unicode_path = {
            let mut file = tempfile::Builder::new().tempfile()?;
            let content = entry.range.as_chars().collect::<String>();
            file.write_all(content.as_bytes())?;
            file.into_temp_path()
        };
        let font_dest = dest_info.as_writable()?;
        let mut p = std::process::Command::new("pyftsubset")
            .arg(ctx.source.file().path())
            .arg(
                [OsStr::new("--text-file="), unicode_path.as_os_str()]
                    .into_iter()
                    .collect::<OsString>(),
            )
            .arg(
                [OsStr::new("--output-file="), font_dest.path().as_ref()]
                    .into_iter()
                    .collect::<OsString>(),
            )
            .arg("--ignore-missing-glyphs")
            .spawn()?;
        let status = p.wait()?;
        if !status.success() {
            bail!("pyftsubset failed with code: {:?}", status.code());
        }
        font_dest.commit()?;
        Ok(())
    }
}

type HistoryFileMap<'a> = HashMap<(&'a Path, Token<'a>), Vec<PathBuf>>;

struct History<'a> {
    files: HistoryFileMap<'a>,
}

impl<'a> History<'a> {
    fn new(contexts: &'a [Context]) -> Result<Self> {
        Ok(Self {
            files: Self::collect_files(contexts)?,
        })
    }

    fn query<'d: 's, 's>(&'s self, dest_info: &'d DestInfo) -> Option<&'s [PathBuf]> {
        let token = FileNameMatcher::new(&dest_info.tmpl.name_template)
            .match_as_token(dest_info.file_path.file_name().unwrap().to_string_lossy())
            .unwrap();
        self.files
            .get(&(&dest_info.tmpl.directory, token))
            .map(AsRef::as_ref)
    }

    fn collect_files(contexts: &'a [Context]) -> Result<HistoryFileMap<'a>> {
        let mut dirs = HashMap::new();
        for ctx in contexts {
            dirs.entry(&ctx.dest_tmpl.directory)
                .or_insert(vec![])
                .push(FileNameMatcher::new(&ctx.dest_tmpl.name_template));
        }
        let mut files = HistoryFileMap::new();
        for (dir, matchers) in dirs {
            let items = fs::read_dir(dir)
                .map_err(|e| anyhow!("Cannot read dir {}: {}", dir.display(), e))?;
            for item in items {
                let item = item?;
                let typ = item.file_type()?;
                if !(typ.is_file() || typ.is_symlink_file()) {
                    continue;
                }
                let mut name = item.file_name().to_string_lossy().into_static();
                for matcher in &matchers {
                    match matcher.match_as_token(name) {
                        Ok(token) => {
                            files
                                .entry((dir, token))
                                .or_insert(vec![])
                                .push(item.path());
                            break;
                        }
                        Err(n) => {
                            name = n;
                        }
                    }
                }
            }
        }
        Ok(files)
    }
}

#[derive(Eq, Hash, PartialEq, Debug)]
struct Token<'a> {
    matcher: FileNameMatcher<'a>,
    mid_prefix: Cow<'a, str>,
}

#[derive(Clone, Default, Eq, Hash, PartialEq, Debug)]
struct FileNameMatcher<'a> {
    prefix: &'a str,
    suffix: &'a str,
}

impl<'a> FileNameMatcher<'a> {
    fn new(tmpl: &'a Tmpl<'static, FontOutputTmplParams>) -> Self {
        let delimiter = FontOutputTmplParams::METAVARS[0];
        let (prefix, suffix) = tmpl.as_str().split_once(delimiter).unwrap();
        Self {
            prefix,
            suffix,
            ..Default::default()
        }
    }
    fn matches<'b>(&self, name: Cow<'b, str>) -> Result<Cow<'b, str>, Cow<'b, str>> {
        match name {
            Cow::Owned(mut name) => {
                let Some(mid) = name.strip_prefix(self.prefix) else {
                    return Err(name.into());
                };
                let Some(mid) = mid.strip_suffix(self.suffix) else {
                    return Err(name.into());
                };
                let Some((mid_prefix, _)) = mid.rsplit_once('_') else {
                    return Err(name.into());
                };
                let mid_prefix_range = name.get_substr_range(mid_prefix).unwrap();
                name.retain_range(mid_prefix_range);
                Ok(name.into())
            }
            Cow::Borrowed(name) => {
                let mid = name
                    .strip_prefix(self.prefix)
                    .ok_or(name)?
                    .strip_suffix(self.suffix)
                    .ok_or(name)?;
                let mid_prefix = mid.rsplit_once('_').ok_or(name)?.0;
                Ok(Cow::Borrowed(mid_prefix))
            }
        }
    }
    fn match_as_token(&self, name: Cow<'a, str>) -> Result<Token<'a>, Cow<'a, str>> {
        let mid_prefix = self.matches(name)?;
        Ok(Token {
            matcher: self.clone(),
            mid_prefix,
        })
    }
}
