use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Parser;
use pulseq_rs::Sequence;

mod viewer;

/// Parse a pulseq .seq file and render it as a standalone HTML viewer.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to the input .seq file.
    input: PathBuf,

    /// Write the rendered HTML to this path. Defaults to a temporary file.
    /// When both viewers are active, "-raw" / "-structured" suffixes are
    /// inserted before the extension.
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Open the rendered HTML in the default browser.
    /// Implied when no --output is given.
    #[arg(long)]
    open: bool,

    /// Don't generate the raw-file viewer.
    #[arg(long)]
    no_raw: bool,

    /// Don't generate the structured-sequence viewer.
    #[arg(long)]
    no_structured: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let source =
        fs::read_to_string(&cli.input).context(format!("reading {}", cli.input.display()))?;
    let sections = pulseq_rs::parse_file(&source).context("parsing input")?;

    if !cli.no_raw {
        let html = viewer::raw::render(&cli.input, &sections);
        write_and_maybe_open(&cli, "raw", &html)?;
    }

    if !cli.no_structured {
        let seq =
            Sequence::from_parsed_file(sections).context("converting sections to sequence")?;
        let html = viewer::structured::render(&cli.input, &seq);
        write_and_maybe_open(&cli, "structured", &html)?;
    }

    Ok(())
}

fn write_and_maybe_open(cli: &Cli, suffix: &str, html: &str) -> anyhow::Result<()> {
    let path = output_path(cli, suffix);
    fs::write(&path, html).context(format!("writing to {}", path.display()))?;
    if cli.open || cli.output.is_none() {
        open::that(&path).context(format!("opening output '{}'", path.display()))?;
    }
    Ok(())
}

fn output_path(cli: &Cli, suffix: &str) -> PathBuf {
    match &cli.output {
        Some(path) => insert_suffix(path, suffix),
        None => {
            std::env::temp_dir().join(format!("pulseq-rs-{}-{}.html", std::process::id(), suffix))
        }
    }
}

fn insert_suffix(path: &Path, suffix: &str) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("html");
    path.with_file_name(format!("{stem}-{suffix}.{ext}"))
}
