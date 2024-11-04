# Pulseq for Rust

This currently only features parsing [pulseq](https://pulseq.github.io/) .seq files.
In the future, functions for building sequences might be added.

# Changelog

### 0.1.2
- Added support for the rfshim pTx extension by loading magnitude and phase shim arrays if found, regardless of file format.

### 0.1.1
- Allow .seq file sections to be empty
- Removed test .seq files - require test-seqs git to be cloned next to pulseq-rs (WIP; might change in the future)

### 0.1.0
Baseline
