# Pulseq for Rust

This crate parses [pulseq](https://pulseq.github.io/) `.seq` files and lowers
them into a representation that mirrors what a scanner actually executes. In
the future, functions for building sequences might be added.

## Crate structure

The crate is organized into three layers, each lifted from the previous one:

- `raw` &mdash; the .seq file as parsed, almost 1:1. Sections, blocks, events,
  shapes and extensions are kept as separate tables indexed by their IDs.
  Supports pulseq 1.2 through 1.5; missing fields from older versions get
  default values, so downstream code does not need to branch on the file
  version.
- `seq` &mdash; an idiomatic, validated representation. IDs are resolved to
  `Arc`-shared references, definitions are parsed, known extensions (labels,
  triggers, soft delays, rotations, RF shims) are recognized, and the
  required-section / event-duration invariants are enforced.
- `int` &mdash; the *interpreted* sequence: what the scanner would actually
  play out. The seq&rarr;int step applies FOV scaling and rotation, folds
  relative and offset RF/ADC freq+phase via the Larmor frequency, applies the
  rotation extension to gradients, resolves soft delays, computes per-ADC
  label snapshots, lifts `ONCE` / `PMC` / triggers into the block, and unifies
  the two possible RF shim sources.

Most users only need `seq` (to inspect a file) or `int` (to know what plays on
the scanner). `raw` is exposed mainly for the file viewer.

## Example: loading a sequence with `int`

```rust
use std::collections::HashMap;
use pulseq_rs::{seq, int};

let seq = seq::Sequence::from_file("example.seq")?;

let (int_seq, warnings) = int::Sequence::from_seq(
    &seq,
    int::Transform::default(),  // identity FOV (scale = 1, no rotation)
    42_577_468.8,               // 1H Larmor frequency @ 1 T [Hz]
    HashMap::new(),             // no soft-delay overrides
)?;

for w in warnings {
    eprintln!("warning: {w}");
}

for block in &int_seq.blocks {
    if let Some(adc) = &block.adc {
        println!(
            "ADC: {} samples, dwell {} s, lin={} par={}",
            adc.num, adc.dwell, adc.labels.lin, adc.labels.par,
        );
    }
}
# Ok::<(), pulseq_rs::Error>(())
```

The standalone `pulseq-rs` binary (built with `--features viewer`) wraps these
three layers into an HTML viewer (`raw` / `structured` / `interpreted`).

## Viewer

Build and run the binary with the `viewer` feature:

```sh
cargo run --features viewer -- example.seq
```

By default this renders one HTML page per layer to a temp file and opens each
in the default browser:

- **raw** &mdash; sections of the parsed .seq file (version, signature,
  definitions, blocks, RFs, gradients, traps, ADCs, delays, extensions,
  shapes) shown as tables with Plotly shape previews on hover.
- **structured** &mdash; the validated `seq::Sequence`: deduplicated events
  (each RF / gradient / ADC / shape gets a stable id), block list,
  definitions, and decoded extensions.
- **interpreted** &mdash; the `int::Sequence` as the scanner would play it:
  one row per block with the FOV-scaled gradients, larmor-folded RF / ADC
  freq+phase, per-ADC label snapshots, triggers, and ONCE / PMC flags
  inlined.

Useful CLI args:

- `-o, --output <FILE>` &mdash; write the HTML next to this path instead of a
  temp file (suffixes `-raw` / `-structured` / `-interp` are inserted).
- `--open` &mdash; force opening in the browser when `--output` is given.
- `--no-raw` / `--no-structured` / `--no-interp` &mdash; skip individual
  views.
- `--larmor <HZ>` &mdash; Larmor frequency used by the interpreter (default:
  1H @ 1 T, `42577468.8`).
- `--soft-delay HINT=SECONDS` &mdash; supply a value for a soft-delay hint;
  repeat for multiple hints.
- `--fov-scale <FACTOR>` &mdash; FOV scale applied by the interpreter
  (default `1.0`).

## Changelog

### 0.2.2

Default all soft delays to 0 if not specified. Emit warnings if missing.

### 0.2.1

Removed the time shape checks

### 0.2.0

Sorted by user-facing impact, biggest first.

- Added Pulseq 1.5 support: RF relative freq/phase, ADC phase shapes, RF `use`
  tag, new complex shapes for RF/shims, explicit time shapes with
  `time_id=-1`, gradient first/last samples (computed for pre-1.5 files).
- Added parsing of all major extensions: Label, Trigger, Delays, Rotations,
  and RF shims.
- Added an `int` module that lowers a structured sequence into an interpreted
  sequence: FOV scale/rotation, larmor-frequency folding of rel. + offset
  freq/phase, label snapshots per ADC, soft-delay resolution, rotation
  extension, ONCE/PMC/triggers on blocks, unified RF shim sources.
- Added a web-based seq file viewer with `raw`, `structured`, and
  `interpreted` views &mdash; RF and arbitrary gradient plotting, blocks,
  definitions, and extension display.
- Replaced the `ezpc` parser with `winnow`, gaining context-rich parse error
  messages.
- Added a `clap`-based CLI for the binary (subcommands, viewer alias).
- **Breaking:** renamed modules `parse_file` &rarr; `raw` and `sequence`
  &rarr; `seq`.
- **Breaking:** removed the `Display` impl for `Seq` (and the `dump_seq` text
  output) &mdash; use the viewer instead.
- Removed `unwrap()`s throughout and surfaced proper error types.
- Refactored raw-to-structured conversion into `seq/convert/`
  (`definitions`, `sections`, `shape_lib`); shape deduplication via a shared
  shape library.
- Compute RF center automatically for pre-1.5 sequences when not provided,
  and preserve it through the structured sequence.
- Improved extension linked-list walking during conversion; split extension
  parsing into separate spec and ref stages; moved `rfuse` parsing into the
  file parsing stage.
- Fixed required-definition and extension validation; label-set errors now
  match the spec / interpreter implementation.
- Fixed default for RF relative freq/phase: `0` (was `1`).
- Added 1.5 test fixtures (`seq_make_radial.seq`) and improved test error
  display.
- Minor: viewer styling/font tweaks, dump_seq example update, `.gitignore` /
  editor config.

### 0.1.3
- Treat rfshim entries with `shape_id = 0` as "no shim" (applies to both v1.2
  and v1.4 files).
- Allow all graphic ASCII symbols in identifiers.

### 0.1.2
- Added support for the rfshim pTx extension by loading magnitude and phase
  shim arrays if found, regardless of file format.

### 0.1.1
- Allow .seq file sections to be empty.
- Removed test .seq files &mdash; require test-seqs git to be cloned next to
  pulseq-rs (WIP; might change in the future).

### 0.1.0
Baseline.
