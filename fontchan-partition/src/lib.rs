mod algorithms;
mod char_base;
mod char_freq;
mod config;
mod pages;

use algorithms::*;

use anyhow::Result;
pub use config::*;
use fontchan_unicode::URange;

pub struct Algorithm {
    ctx: AlgorithmContext,
    impl_: Box<dyn AlgorithmImpl>,
}

impl Algorithm {
    pub fn partition(&self) -> Vec<URange> {
        self.impl_.partition(&self.ctx)
    }
}

pub fn build_algorithm(context: &Context, config: &Config) -> Result<Algorithm> {
    let char_base = char_base::CHAR_BASE_REGISTRY
        .build(context, &config.char_base)?
        .into_data();
    let char_freq = char_freq::CHAR_FREQ_REGISTRY
        .build(context, &config.char_freq)?
        .into_data();
    let pages = pages::PAGES_REGISTRY
        .build(context, &config.pages)?
        .into_data();
    let algo_ctx = AlgorithmContext {
        part_size: config.part_size,
        char_base,
        char_freq,
        pages,
    };
    let impl_ = ALGORITHM_REGISTRY
        .build(&algo_ctx, &config.algorithm)?
        .into_data();
    Ok(Algorithm {
        ctx: algo_ctx,
        impl_,
    })
}

#[test]
fn test() {
    use fontchan_util::routine;
    use std::sync::Arc;

    let context = Context {
        font_files: vec![Arc::new("samples/LXGWWenKaiGB-Regular.woff".into())],
        ..Default::default()
    };
    let config = Config {
        part_size: Default::default(),
        char_base: Some(routine!("from_fonts").into()),
        char_freq: Some(routine!("preset_zh").into()),
        pages: Some(routine!("glob[../../hsfzxjy.github.io/public/**/*.html]").into()),
        algorithm: Default::default(),
    };
    let algo = build_algorithm(&context, &config).unwrap();
    let res = algo.partition();
    dbg!(res[0..10]
        .iter()
        .map(|x| x.as_chars().collect::<String>())
        .collect::<Vec<_>>());
}
