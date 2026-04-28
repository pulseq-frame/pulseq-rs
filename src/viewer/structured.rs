use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;
use std::sync::Arc;

use maud::{Markup, PreEscaped, html};
use pulseq_rs::{Adc, Block, Extension, Gradient, Rf, Sequence, Shape};

use crate::viewer::{fmt_seconds, json_floats, page};

/// Per-type identity counters. Each unique `Arc<T>` (compared by pointer) gets
/// a stable id assigned the first time we see it; subsequent occurrences reuse
/// the same id. Display ids are 1-indexed.
///
/// `shape_data` collects `shapes[id] = [...]` JS assignments and `shape_divs`
/// collects `<div id='shape-plot-id'></div>` placeholders — one of each per
/// unique shape. JS later moves these divs into popups on hover and renders
/// the Plotly chart lazily.
#[derive(Default)]
struct Counters {
    rf: HashMap<*const Rf, u32>,
    grad: HashMap<*const Gradient, u32>,
    adc: HashMap<*const Adc, u32>,
    shape: HashMap<*const Shape, u32>,
    shape_data: String,
    shape_divs: String,
}

impl Counters {
    fn rf(&mut self, x: &Arc<Rf>) -> u32 {
        let next = self.rf.len() as u32 + 1;
        *self.rf.entry(Arc::as_ptr(x)).or_insert(next)
    }
    fn grad(&mut self, x: &Arc<Gradient>) -> u32 {
        let next = self.grad.len() as u32 + 1;
        *self.grad.entry(Arc::as_ptr(x)).or_insert(next)
    }
    fn adc(&mut self, x: &Arc<Adc>) -> u32 {
        let next = self.adc.len() as u32 + 1;
        *self.adc.entry(Arc::as_ptr(x)).or_insert(next)
    }
    fn shape(&mut self, x: &Arc<Shape>) -> u32 {
        let ptr = Arc::as_ptr(x);
        if let Some(&existing) = self.shape.get(&ptr) {
            return existing;
        }
        let id = self.shape.len() as u32 + 1;
        self.shape.insert(ptr, id);
        let _ = writeln!(self.shape_data, "shapes[{id}] = {};", json_floats(&x.0));
        let _ = writeln!(self.shape_divs, "<div id='shape-plot-{id}'></div>");
        id
    }
}

pub fn render(input: &Path, seq: &Sequence) -> String {
    let mut counters = Counters::default();
    let title = input.display().to_string();

    let body = html! {
        h1 { (title) }
        p.meta {
            (seq.blocks.len()) " blocks · total duration "
            (fmt_seconds(seq.blocks.iter().map(|b| b.duration).sum()))
        }

        section {
            h2 { "Definitions" }
            (render_definitions(seq))
        }

        section {
            h2 { "Sequence" }
            (render_sequence(seq, &mut counters))
        }

        div #shape-registry style="display:none" {
            (PreEscaped(counters.shape_divs))
        }
    };

    let inline_script = html! {
        "var shapes = {};\n"
        (PreEscaped(counters.shape_data))
    };

    page(&title, "viewer-structured", body, inline_script)
}

fn render_definitions(seq: &Sequence) -> Markup {
    html! {
        h3 { "Time raster" }
        table.kv { tbody {
            @for (label, value) in [
                ("Gradient", seq.time_raster.grad),
                ("RF", seq.time_raster.rf),
                ("ADC", seq.time_raster.adc),
                ("Block", seq.time_raster.block),
            ] {
                tr { td { (label) } td { (fmt_seconds(value)) } }
            }
        } }

        h3 { "FOV" }
        @match seq.fov {
            Some((x, y, z)) => p {
                (format!("{:.1}", x * 1e3)) " × "
                (format!("{:.1}", y * 1e3)) " × "
                (format!("{:.1}", z * 1e3)) " mm"
            },
            None => p.empty { "(unset)" },
        }

        h3 { "Name" }
        @match &seq.name {
            Some(n) => p { (n) },
            None => p.empty { "(unset)" },
        }

        h3 { "Other definitions" }
        (render_other_definitions(seq))
    }
}

fn render_other_definitions(seq: &Sequence) -> Markup {
    if seq.definitions.is_empty() {
        return html! { p.empty { "(none)" } };
    }
    // Sort for deterministic output (HashMap iteration is unordered).
    let mut entries: Vec<_> = seq.definitions.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    html! {
        table.kv { tbody {
            @for (k, v) in entries {
                tr { td { (k) } td { (v) } }
            }
        } }
    }
}

fn render_sequence(seq: &Sequence, counters: &mut Counters) -> Markup {
    if seq.blocks.is_empty() {
        return html! { p.empty { "(no blocks)" } };
    }
    html! {
        table.sequence { tbody {
            @for block in &seq.blocks {
                tr {
                    td { (block.id) }
                    td { (fmt_seconds(block.duration)) }
                    td { (block_events(block, counters)) }
                }
            }
        } }
    }
}

fn block_events(block: &Block, counters: &mut Counters) -> Markup {
    html! {
        @if let Some(rf) = &block.rf { (render_rf_tag(rf, counters)) " " }
        @for (axis, grad) in [("GX", &block.gx), ("GY", &block.gy), ("GZ", &block.gz)] {
            @if let Some(grad) = grad { (render_grad_tag(axis, grad, counters)) " " }
        }
        @if let Some(adc) = &block.adc { (render_adc_tag(adc, counters)) " " }
        @if !block.ext.is_empty() { (render_ext_tag(&block.ext)) }
    }
}

fn render_rf_tag(rf: &Arc<Rf>, counters: &mut Counters) -> Markup {
    let id = counters.rf(rf);
    html! {
        span.rf-tag {
            (format!("<RF_{id:02X}>"))
            span.ext-popup { ul {
                li { strong { "amp" } (rf.amp) " Hz" }
                li { strong { "phase" } (rf.phase) " rad" }
                li { strong { "delay" } (fmt_seconds(rf.delay)) }
                li { strong { "freq" } (rf.freq) " Hz" }
                li { strong { "amp shape" } (render_shape_link(&rf.amp_shape, counters)) }
                li { strong { "phase shape" } (render_shape_link(&rf.phase_shape, counters)) }
                @if let Some((mag, phase)) = &rf.shim_shape {
                    li { strong { "shim mag" } (render_shape_link(mag, counters)) }
                    li { strong { "shim phase" } (render_shape_link(phase, counters)) }
                }
            } }
        }
    }
}

fn render_shape_link(shape: &Arc<Shape>, counters: &mut Counters) -> Markup {
    let id = counters.shape(shape);
    let n = shape.0.len();
    html! {
        span.shape-link data-shape=(id) {
            (format!("shape {id:02x} ({n} samples)"))
            span.shape-popup {}
        }
    }
}

fn render_grad_tag(axis: &str, grad: &Arc<Gradient>, counters: &mut Counters) -> Markup {
    let id = counters.grad(grad);
    match grad.as_ref() {
        Gradient::Free { amp, delay, shape } => html! {
            span.free-tag {
                (format!("<{axis}_{id:02X}>"))
                span.ext-popup { ul {
                    li { strong { "amp" } (amp) " Hz/m" }
                    li { strong { "delay" } (fmt_seconds(*delay)) }
                    li { strong { "shape" } (render_shape_link(shape, counters)) }
                } }
            }
        },
        Gradient::Trap {
            amp,
            rise,
            flat,
            fall,
            delay,
        } => html! {
            span.trap-tag {
                (format!("<{axis}_{id:02X}>"))
                span.ext-popup { ul {
                    li { strong { "amp" } (amp) " Hz/m" }
                    li { strong { "rise" } (fmt_seconds(*rise)) }
                    li { strong { "flat" } (fmt_seconds(*flat)) }
                    li { strong { "fall" } (fmt_seconds(*fall)) }
                    li { strong { "delay" } (fmt_seconds(*delay)) }
                } }
            }
        },
    }
}

fn render_adc_tag(adc: &Arc<Adc>, counters: &mut Counters) -> Markup {
    let id = counters.adc(adc);
    html! {
        span.adc-tag {
            (format!("<ADC_{id:02X}>"))
            span.ext-popup { ul {
                li { strong { "num" } (adc.num) }
                li { strong { "dwell" } (fmt_seconds(adc.dwell)) }
                li { strong { "delay" } (fmt_seconds(adc.delay)) }
                li { strong { "freq" } (adc.freq) " Hz" }
                li { strong { "phase" } (adc.phase) " rad" }
            } }
        }
    }
}

struct ExtensionRender<'a>(&'a Extension);

#[cfg(feature = "viewer")]
impl<'a> maud::Render for ExtensionRender<'a> {
    fn render(&self) -> maud::Markup {
        match &self.0 {
            Extension::Unsupported { string_id, data } => maud::html! {
                li { em.ext-unsupported { "\"" (string_id) "\"" } (data) }
            },
            Extension::LabelSet { flag, value } => maud::html! {
                li { strong { "LABELSET" } code { (flag) " = " (value) } }
            },
            Extension::LabelInc { counter, value } => maud::html! {
                li { strong { "LABELINC" } code { (counter) " += " (value) } }
            },
        }
    }
}

fn render_ext_tag(ext: &[Extension]) -> Markup {
    html! {
        span.ext-tag {
            "<EXT>"
            span.ext-popup { ul {
                @for item in ext { (ExtensionRender(item)) }
            } }
        }
    }
}
