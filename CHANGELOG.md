# Changelog

## dev0.2-no-ezpc (unreleased)

Sorted by user-facing impact, biggest first.

- Added Pulseq 1.5 support: RF relative freq/phase, ADC phase shapes, RF `use` tag, new complex shapes for RF/shims, explicit time shapes with `time_id=-1`.
- Added parsing of all major extensions: Label, Trigger, Delays, Rotations, and RF shims.
- Added a web-based seq file viewer (`raw` and `structured` views) with RF, arbitrary gradient plotting, blocks, definitions, and extension display.
- Replaced the `ezpc` parser with `winnow`, gaining context-rich parse error messages.
- Added a `clap`-based CLI for the binary (subcommands, viewer alias).
- **Breaking:** renamed modules `parse_file` → `raw` and `sequence` → `seq`.
- **Breaking:** removed the `Display` impl for `Seq` (and the `dump_seq` text output) — use the viewer instead.
- Removed `unwrap()`s throughout and surfaced proper error types.
- Refactored raw-to-structured conversion into `seq/convert/` (`definitions`, `sections`, `shape_lib`).
- Started new `int` module: outline for an interpreted sequence (WIP).
- Compute RF center automatically for pre-1.5 sequences when not provided.
- Preserve RF center in structured sequence (no longer dropped).
- Improved extension linked-list walking during conversion.
- Moved `rfuse` parsing into file parsing stage.
- Fixed required definitions and extension validation.
- Added 1.5 test fixtures (`seq_make_radial.seq`) and improved test error display.
- Minor: viewer styling/font tweaks, dump_seq example update, `.gitignore`/editor config.
