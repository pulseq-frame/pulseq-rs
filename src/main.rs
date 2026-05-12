use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Parser;
use pulseq_rs::seq::Sequence;

mod viewer;

/// Parse a pulseq .seq file and render it as a standalone HTML viewer.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to the input .seq file.
    input: PathBuf,

    /// Write the rendered HTML to this path. Defaults to a temporary file.
    /// When several viewers are active, "-raw" / "-structured" / "-interp"
    /// suffixes are inserted before the extension.
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

    /// Don't generate the interpreted-sequence viewer.
    #[arg(long)]
    no_interp: bool,

    /// Larmor frequency [Hz] used by the interpreter to fold relative and
    /// offset components of RF / ADC freq and phase. Default: 1H at 1 T.
    #[arg(long, default_value_t = 42_577_468.8, value_name = "HZ")]
    larmor: f64,

    /// Soft-delay value, repeatable as `hint=value` (value in seconds).
    /// Required for any soft-delay hint referenced by the sequence.
    #[arg(long = "soft-delay", value_name = "HINT=SECONDS")]
    soft_delay: Vec<String>,

    /// FOV scale factor applied by the interpreter. Other FOV components
    /// (rotation, position) keep their identity defaults.
    #[arg(long, default_value_t = 1.0, value_name = "FACTOR")]
    fov_scale: f64,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let source =
        fs::read_to_string(&cli.input).context(format!("reading {}", cli.input.display()))?;
    let sections = pulseq_rs::raw::parse_file(&source).context("parsing input")?;

    if !cli.no_raw {
        let html = viewer::raw::render(&cli.input, &sections);
        write_and_maybe_open(&cli, "raw", &html)?;
    }

    if !cli.no_structured || !cli.no_interp {
        let seq =
            Sequence::from_parsed_file(sections).context("converting sections to sequence")?;

        if !cli.no_structured {
            let html = viewer::structured::render(&cli.input, &seq);
            write_and_maybe_open(&cli, "structured", &html)?;
        }

        if !cli.no_interp {
            run_interp_viewer(&cli, &seq)?;
        }
    }

    Ok(())
}

fn run_interp_viewer(cli: &Cli, seq: &Sequence) -> anyhow::Result<()> {
    let fov = pulseq_rs::int::Transform {
        scale: cli.fov_scale,
        ..Default::default()
    };
    let soft_delays = parse_soft_delays(&cli.soft_delay)?;

    match pulseq_rs::int::Sequence::from_seq(seq, fov, cli.larmor, soft_delays) {
        Ok((int_seq, warnings)) => {
            for w in warnings {
                eprintln!("warning: {w}");
            }
            let html = viewer::int::render(&cli.input, &int_seq);
            write_and_maybe_open(cli, "interp", &html)?;
        }
        Err(e) => {
            eprintln!("skipping interp viewer: {e}");
        }
    }
    Ok(())
}

fn parse_soft_delays(args: &[String]) -> anyhow::Result<HashMap<String, f64>> {
    let mut out = HashMap::new();
    for entry in args {
        let (hint, value) = entry
            .split_once('=')
            .with_context(|| format!("--soft-delay must be 'hint=value', got {entry:?}"))?;
        let parsed: f64 = value
            .parse()
            .with_context(|| format!("--soft-delay {hint:?}: value '{value}' is not a number"))?;
        out.insert(hint.to_string(), parsed);
    }
    Ok(out)
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
