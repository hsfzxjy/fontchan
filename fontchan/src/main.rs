use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use config::Config;
use fontchan_unicode::UEntry;
use fontchan_util::{CloneS, WorkDir};
use rayon::prelude::*;
use serde::Deserialize;

mod builder;
mod config;

#[derive(clap::Parser, Debug)]
struct Cli {
    #[clap(help = "The config TOML file")]
    config_path: PathBuf,
}

fn main() -> Result<()> {
    run_main(Cli::parse())
}

fn run_main(cli: Cli) -> Result<()> {
    let config_content = std::fs::read_to_string(&cli.config_path)?;
    let deserializer = toml::Deserializer::new(&config_content);
    WorkDir::init_global(None, &cli.config_path, deserializer)?;
    let config = Config::deserialize(toml::Deserializer::new(&config_content))?;

    let partition = {
        use fontchan_partition::{build_algorithm, Context};
        let context = Context {
            font_files: config.fonts.iter().map(|f| f.input_path.clone()).collect(),
            ..Default::default()
        };
        let config = &config.partition;
        build_algorithm(&context, &config)?.partition()
    };

    let entries = partition
        .iter()
        .enumerate()
        .map(|(i, range)| UEntry {
            name: i.to_string().into(),
            range,
        })
        .collect::<Vec<_>>();

    let result =
        builder::FontBuilder::new(&config)?.build(entries.par_iter().map(CloneS::clone_s))?;

    builder::JSBuilder.build(
        config.builder.js.output_path.into(),
        config.fonts.iter().map(|f| &f.css),
        entries.iter().map(|e| e.range),
        &result,
    )?;
    Ok(())
}

#[test]
fn test() -> Result<()> {
    run_main(Cli::parse_from(vec![
        "fontchan",
        "../samples/fontchan.config.toml",
    ]))
}
