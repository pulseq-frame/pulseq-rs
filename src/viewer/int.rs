use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;
use std::sync::Arc;

use maud::{Markup, PreEscaped, html};
use num_complex::Complex64;
use pulseq_rs::int::{
    Adc, Block, BlockLabels, Gradient, Labels, Once, Rf, Sequence, Shape, Trigger,
};

use crate::viewer::{fmt_seconds, json_floats, page};

/// Per-shape identity counter. Only shapes are still `Arc`-shared in `int`,
/// so unlike the structured viewer's `Counters` we don't keep maps for
/// rf/grad/adc — those are owned inline and rendered without ids.
///
/// `shape_data` collects `shapes[id] = …` JS assignments; `shape_divs`
/// collects matching `<div id='shape-plot-id'></div>` placeholders. The JS
/// in `viewer.js` keys off `data-shape` / `data-cshape` attributes and
/// these placeholder ids, so the inline format here matches the structured
/// viewer exactly.
#[derive(Default)]
struct Counters {
    shape: HashMap<*const Shape<f64>, u32>,
    cshape: HashMap<*const Shape<Complex64>, u32>,
    shape_data: String,
    shape_divs: String,
}

impl Counters {
    fn shape(&mut self, x: &Arc<Shape<f64>>) -> u32 {
        let ptr = Arc::as_ptr(x);
        if let Some(&existing) = self.shape.get(&ptr) {
            return existing;
        }
        let id = (self.shape.len() + self.cshape.len()) as u32 + 1;
        self.shape.insert(ptr, id);
        let _ = writeln!(
            self.shape_data,
            "shapes[{id}] = {{time: {}, amp: {}}};",
            json_floats(&x.time),
            json_floats(&x.amp),
        );
        let _ = writeln!(self.shape_divs, "<div id='shape-plot-{id}'></div>");
        id
    }

    fn complex_shape(&mut self, x: &Arc<Shape<Complex64>>) -> u32 {
        let ptr = Arc::as_ptr(x);
        if let Some(&existing) = self.cshape.get(&ptr) {
            return existing;
        }
        let id = (self.shape.len() + self.cshape.len()) as u32 + 1;
        let re: Vec<f64> = x.amp.iter().map(|c| c.re).collect();
        let im: Vec<f64> = x.amp.iter().map(|c| c.im).collect();
        let _ = writeln!(
            self.shape_data,
            "cshapes[{id}] = {{time: {}, re: {}, im: {}}};",
            json_floats(&x.time),
            json_floats(&re),
            json_floats(&im),
        );
        let _ = writeln!(self.shape_divs, "<div id='cshape-plot-{id}'></div>");
        self.cshape.insert(ptr, id);
        id
    }
}

pub fn render(input: &Path, seq: &Sequence) -> String {
    let mut counters = Counters::default();
    let title = input.display().to_string();
    let total: f64 = seq.blocks.iter().map(|b| b.duration).sum();

    let body = html! {
        h1 { (title) }
        p.meta {
            (seq.blocks.len()) " blocks · total duration "
            (fmt_seconds(total))
        }

        section {
            h2 { "Sequence info" }
            (render_info(seq))
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
        "var shapes = {};\nvar cshapes = {};\n"
        (PreEscaped(counters.shape_data))
    };

    page(&title, "viewer-interp", body, inline_script)
}

fn render_info(seq: &Sequence) -> Markup {
    html! {
        h3 { "Name" }
        @match &seq.name {
            Some(n) => p { (n) },
            None => p.empty { "(unset)" },
        }

        h3 { "FOV" }
        p {
            (format!("{:.1}", seq.fov[0] * 1e3)) " × "
            (format!("{:.1}", seq.fov[1] * 1e3)) " × "
            (format!("{:.1}", seq.fov[2] * 1e3)) " mm"
        }
    }
}

fn render_sequence(seq: &Sequence, counters: &mut Counters) -> Markup {
    if seq.blocks.is_empty() {
        return html! { p.empty { "(no blocks)" } };
    }
    html! {
        table.sequence { tbody {
            @for (i, block) in seq.blocks.iter().enumerate() {
                tr {
                    td { (i + 1) }
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
        @if !block.triggers.is_empty() { (render_triggers_tag(&block.triggers)) " " }
        @if !is_default_block_labels(&block.labels) { (render_block_labels_tag(&block.labels)) }
    }
}

fn render_rf_tag(rf: &Rf, counters: &mut Counters) -> Markup {
    html! {
        span.rf-tag {
            "<RF>"
            span.ext-popup { ul {
                li { strong { "amp" } (rf.amp) " Hz" }
                li { strong { "phase" } (rf.phase) " rad" }
                li { strong { "delay" } (fmt_seconds(rf.delay)) }
                li { strong { "center" } (fmt_seconds(rf.center)) }
                li { strong { "freq" } (rf.freq) " Hz" }
                li { strong { "shape" } (render_complex_shape_link(&rf.shape, counters)) }
                li { strong { "shims" } code { (format_shims(&rf.shims)) } }
                li { strong { "use" } (rf.rf_use) }
            } }
        }
    }
}

fn format_shims(shims: &[Complex64]) -> String {
    let mut out = String::new();
    for (i, c) in shims.iter().enumerate() {
        let _ = write!(
            out,
            " ch{}=({:.3}, {:.1}°)",
            i + 1,
            c.norm(),
            c.arg().to_degrees(),
        );
    }
    out
}

fn render_grad_tag(axis: &str, grad: &Gradient, counters: &mut Counters) -> Markup {
    html! {
        span.free-tag {
            (format!("<{axis}>"))
            span.ext-popup { ul {
                li { strong { "amp" } (grad.amp) " Hz/m" }
                li { strong { "delay" } (fmt_seconds(grad.delay)) }
                li { strong { "shape" } (render_shape_link(&grad.shape, counters)) }
            } }
        }
    }
}

fn render_adc_tag(adc: &Adc, counters: &mut Counters) -> Markup {
    html! {
        span.adc-tag {
            "<ADC>"
            span.ext-popup { ul {
                li { strong { "num" } (adc.num) }
                li { strong { "dwell" } (fmt_seconds(adc.dwell)) }
                li { strong { "delay" } (fmt_seconds(adc.delay)) }
                li { strong { "freq" } (adc.freq) " Hz" }
                li { strong { "phase" } (adc.phase) " rad" }
                @if let Some(ps) = &adc.phase_shape {
                    li { strong { "phase shape" } (render_shape_link(ps, counters)) }
                }
                @if !is_default_labels(&adc.labels) {
                    li { strong { "labels" } code { (format_labels(&adc.labels)) } }
                }
            } }
        }
    }
}

fn render_triggers_tag(triggers: &[Trigger]) -> Markup {
    html! {
        span.ext-tag {
            "<TRG>"
            span.ext-popup { ul {
                @for t in triggers {
                    li { code {
                        "type=" (t.typ) ", channel=" (t.channel)
                        ", delay=" (fmt_seconds(t.delay))
                        ", duration=" (fmt_seconds(t.duration))
                    } }
                }
            } }
        }
    }
}

fn render_block_labels_tag(labels: &BlockLabels) -> Markup {
    html! {
        span.ext-tag {
            "<LBL>"
            span.ext-popup { ul {
                @if !matches!(labels.once, Once::Always) {
                    li { strong { "once" } (once_str(labels.once)) }
                }
                @if labels.pmc { li { strong { "pmc" } "true" } }
                @if labels.trid != 0 { li { strong { "trid" } (labels.trid) } }
            } }
        }
    }
}

fn once_str(once: Once) -> &'static str {
    match once {
        Once::Always => "Always",
        Once::First => "First",
        Once::Last => "Last",
    }
}

fn is_default_block_labels(l: &BlockLabels) -> bool {
    matches!(l.once, Once::Always) && !l.pmc && l.trid == 0
}

fn is_default_labels(l: &Labels) -> bool {
    l.slc == 0
        && l.seg == 0
        && l.rep == 0
        && l.avg == 0
        && l.set == 0
        && l.eco == 0
        && l.phs == 0
        && l.lin == 0
        && l.par == 0
        && l.acq == 0
        && !l.nav
        && !l.rev
        && !l.sms
        && !l.ref_
        && !l.ima
        && !l.off
        && !l.noise
}

fn format_labels(l: &Labels) -> String {
    let mut parts: Vec<String> = Vec::new();
    let counters = [
        ("slc", l.slc),
        ("seg", l.seg),
        ("rep", l.rep),
        ("avg", l.avg),
        ("set", l.set),
        ("eco", l.eco),
        ("phs", l.phs),
        ("lin", l.lin),
        ("par", l.par),
        ("acq", l.acq),
    ];
    for (name, value) in counters {
        if value != 0 {
            parts.push(format!("{name}={value}"));
        }
    }
    let bools = [
        ("nav", l.nav),
        ("rev", l.rev),
        ("sms", l.sms),
        ("ref", l.ref_),
        ("ima", l.ima),
        ("off", l.off),
        ("noise", l.noise),
    ];
    for (name, on) in bools {
        if on {
            parts.push(name.to_string());
        }
    }
    parts.join(" ")
}

fn render_shape_link(shape: &Arc<Shape<f64>>, counters: &mut Counters) -> Markup {
    let id = counters.shape(shape);
    let n = shape.amp.len();
    html! {
        span.shape-link data-shape=(id) {
            (format!("shape {id:02x} ({n} samples)"))
            span.shape-popup {}
        }
    }
}

fn render_complex_shape_link(shape: &Arc<Shape<Complex64>>, counters: &mut Counters) -> Markup {
    let id = counters.complex_shape(shape);
    let n = shape.amp.len();
    html! {
        span.shape-link data-cshape=(id) {
            (format!("shape {id:02x} ({n} samples)"))
            span.shape-popup {}
        }
    }
}
