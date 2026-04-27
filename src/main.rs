use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

mod viewer;

/// Parse a pulseq .seq file and render it as a standalone HTML viewer.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to the input .seq file.
    input: PathBuf,

    /// Write the rendered HTML to this path. Defaults to a temporary file.
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Open the rendered HTML in the default browser.
    /// Implied when no --output is given.
    #[arg(long)]
    open: bool,
}

fn main() -> anyhow::Result<()> {
    // Parse cli parameters and try to read in required input file
    let cli = Cli::parse();
    let source =
        fs::read_to_string(&cli.input).context(format!("reading {}", cli.input.display()))?;

    let sections = pulseq_rs::parse_file(&source).context("parsing input")?;
    let html = viewer::render(&cli.input, &sections);

    // Write either to the given output file or to a temporary file for viewing
    let path = cli.output.clone().unwrap_or_else(|| {
        std::env::temp_dir().join(format!("pulseq-rs-{}.html", std::process::id()))
    });
    std::fs::write(&path, html).context(format!("writing to {}", path.display()))?;

    // Open if --open flag was set or if no output file was specified
    if cli.open || cli.output.is_none() {
        open::that(&path).context(format!("opening output '{}'", path.display()))?;
    }

    Ok(())
}
