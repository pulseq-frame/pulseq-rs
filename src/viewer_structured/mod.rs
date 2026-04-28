use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;
use std::sync::Arc;

use pulseq_rs::{Adc, Block, Gradient, Rf, Sequence, Shape};

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
    /// Assign / look up a shape's display id; on first sight, emit its sample
    /// array and a placeholder `<div>` for the chart. Subsequent calls just
    /// return the existing id.
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

const TEMPLATE: &str = include_str!("template.html");

pub fn render(input: &Path, seq: &Sequence) -> String {
    let mut counters = Counters::default();
    let sequence_html = render_sequence(seq, &mut counters);

    TEMPLATE
        .replace("__TITLE__", &escape(&input.display().to_string()))
        .replace("__META__", &render_meta(seq))
        .replace("__DEFINITIONS__", &render_definitions(seq))
        .replace("__SEQUENCE__", &sequence_html)
        .replace("/*__SHAPE_DATA__*/", &counters.shape_data)
        .replace("__SHAPE_DIVS__", &counters.shape_divs)
}

fn render_meta(seq: &Sequence) -> String {
    let total: f64 = seq.blocks.iter().map(|b| b.duration).sum();
    format!(
        "{} blocks &middot; total duration {}",
        seq.blocks.len(),
        fmt_seconds(total)
    )
}

fn render_definitions(seq: &Sequence) -> String {
    let mut out = String::new();

    out.push_str("<h3>Time raster</h3>");
    out.push_str("<table class='kv'><tbody>");
    for (label, value) in [
        ("Gradient", seq.time_raster.grad),
        ("RF", seq.time_raster.rf),
        ("ADC", seq.time_raster.adc),
        ("Block", seq.time_raster.block),
    ] {
        let _ = write!(
            out,
            "<tr><td>{label}</td><td>{}</td></tr>",
            fmt_seconds(value)
        );
    }
    out.push_str("</tbody></table>");

    out.push_str("<h3>FOV</h3>");
    match seq.fov {
        Some((x, y, z)) => {
            let _ = write!(
                out,
                "<p>{:.1} &times; {:.1} &times; {:.1} mm</p>",
                x * 1e3,
                y * 1e3,
                z * 1e3
            );
        }
        None => out.push_str(r#"<p class="empty">(unset)</p>"#),
    }

    out.push_str("<h3>Name</h3>");
    match &seq.name {
        Some(n) => {
            let _ = write!(out, "<p>{}</p>", escape(n));
        }
        None => out.push_str(r#"<p class="empty">(unset)</p>"#),
    }

    out.push_str("<h3>Other definitions</h3>");
    if seq.definitions.is_empty() {
        out.push_str(r#"<p class="empty">(none)</p>"#);
    } else {
        // Sort for deterministic output (HashMap iteration is unordered).
        let mut entries: Vec<_> = seq.definitions.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));

        out.push_str("<table class='kv'><tbody>");
        for (k, v) in entries {
            let _ = write!(
                out,
                "<tr><td>{}</td><td>{}</td></tr>",
                escape(k),
                escape(v)
            );
        }
        out.push_str("</tbody></table>");
    }

    out
}

fn render_sequence(seq: &Sequence, counters: &mut Counters) -> String {
    if seq.blocks.is_empty() {
        return r#"<p class="empty">(no blocks)</p>"#.to_string();
    }

    let mut out = String::from("<table class='sequence'><tbody>");
    for block in &seq.blocks {
        let _ = write!(
            out,
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
            block.id,
            fmt_seconds(block.duration),
            block_events(block, counters),
        );
    }
    out.push_str("</tbody></table>");
    out
}

fn block_events(block: &Block, counters: &mut Counters) -> String {
    let mut tags: Vec<String> = Vec::new();
    if let Some(rf) = &block.rf {
        tags.push(render_rf_tag(rf, counters));
    }
    for (axis, grad) in [("GX", &block.gx), ("GY", &block.gy), ("GZ", &block.gz)] {
        if let Some(grad) = grad {
            tags.push(render_grad_tag(axis, grad, counters));
        }
    }
    if let Some(adc) = &block.adc {
        tags.push(render_adc_tag(adc, counters));
    }
    if !block.ext.is_empty() {
        tags.push(render_ext_tag(&block.ext));
    }
    tags.join(" ")
}

fn render_rf_tag(rf: &Arc<Rf>, counters: &mut Counters) -> String {
    let id = counters.rf(rf);
    let mut popup = String::from("<span class='ext-popup'><ul>");
    let _ = write!(popup, "<li><strong>amp</strong>{} Hz</li>", rf.amp);
    let _ = write!(popup, "<li><strong>phase</strong>{} rad</li>", rf.phase);
    let _ = write!(
        popup,
        "<li><strong>delay</strong>{}</li>",
        fmt_seconds(rf.delay)
    );
    let _ = write!(popup, "<li><strong>freq</strong>{} Hz</li>", rf.freq);
    let _ = write!(
        popup,
        "<li><strong>amp shape</strong>{}</li>",
        render_shape_link(&rf.amp_shape, counters),
    );
    let _ = write!(
        popup,
        "<li><strong>phase shape</strong>{}</li>",
        render_shape_link(&rf.phase_shape, counters),
    );
    if let Some((mag, phase)) = &rf.shim_shape {
        let _ = write!(
            popup,
            "<li><strong>shim mag</strong>{}</li>",
            render_shape_link(mag, counters),
        );
        let _ = write!(
            popup,
            "<li><strong>shim phase</strong>{}</li>",
            render_shape_link(phase, counters),
        );
    }
    popup.push_str("</ul></span>");
    format!("<span class='rf-tag'>&lt;RF_{id:02X}&gt;{popup}</span>")
}

fn render_shape_link(shape: &Arc<Shape>, counters: &mut Counters) -> String {
    let id = counters.shape(shape);
    format!(
        "<span class='shape-link' data-shape='{id}'>shape {id:02x} ({n} samples)\
          <span class='shape-popup'></span>\
        </span>",
        n = shape.0.len(),
    )
}

fn render_grad_tag(axis: &str, grad: &Arc<Gradient>, counters: &mut Counters) -> String {
    let id = counters.grad(grad);
    match grad.as_ref() {
        Gradient::Free { amp, delay, shape } => {
            let mut popup = String::from("<span class='ext-popup'><ul>");
            let _ = write!(popup, "<li><strong>amp</strong>{amp} Hz/m</li>");
            let _ = write!(
                popup,
                "<li><strong>delay</strong>{}</li>",
                fmt_seconds(*delay)
            );
            let _ = write!(
                popup,
                "<li><strong>shape</strong>{}</li>",
                render_shape_link(shape, counters),
            );
            popup.push_str("</ul></span>");
            format!("<span class='free-tag'>&lt;{axis}_{id:02X}&gt;{popup}</span>")
        }
        Gradient::Trap {
            amp,
            rise,
            flat,
            fall,
            delay,
        } => {
            let mut popup = String::from("<span class='ext-popup'><ul>");
            let _ = write!(popup, "<li><strong>amp</strong>{amp} Hz/m</li>");
            let _ = write!(
                popup,
                "<li><strong>rise</strong>{}</li>",
                fmt_seconds(*rise)
            );
            let _ = write!(
                popup,
                "<li><strong>flat</strong>{}</li>",
                fmt_seconds(*flat)
            );
            let _ = write!(
                popup,
                "<li><strong>fall</strong>{}</li>",
                fmt_seconds(*fall)
            );
            let _ = write!(
                popup,
                "<li><strong>delay</strong>{}</li>",
                fmt_seconds(*delay)
            );
            popup.push_str("</ul></span>");
            format!("<span class='trap-tag'>&lt;{axis}_{id:02X}&gt;{popup}</span>")
        }
    }
}

fn json_floats(xs: &[f64]) -> String {
    let mut s = String::with_capacity(xs.len() * 6);
    s.push('[');
    for (i, x) in xs.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        if x.is_finite() {
            let _ = write!(s, "{x}");
        } else {
            s.push_str("null");
        }
    }
    s.push(']');
    s
}

fn render_adc_tag(adc: &Arc<Adc>, counters: &mut Counters) -> String {
    let id = counters.adc(adc);
    let mut popup = String::from("<span class='ext-popup'><ul>");
    let _ = write!(popup, "<li><strong>num</strong>{}</li>", adc.num);
    let _ = write!(
        popup,
        "<li><strong>dwell</strong>{}</li>",
        fmt_seconds(adc.dwell)
    );
    let _ = write!(
        popup,
        "<li><strong>delay</strong>{}</li>",
        fmt_seconds(adc.delay)
    );
    let _ = write!(popup, "<li><strong>freq</strong>{} Hz</li>", adc.freq);
    let _ = write!(popup, "<li><strong>phase</strong>{} rad</li>", adc.phase);
    popup.push_str("</ul></span>");
    format!("<span class='adc-tag'>&lt;ADC_{id:02X}&gt;{popup}</span>")
}

fn render_ext_tag(ext: &[(String, String)]) -> String {
    let mut popup = String::from("<span class='ext-popup'><ul>");
    for (name, data) in ext {
        let _ = write!(
            popup,
            "<li><strong>{}</strong>{}</li>",
            escape(name),
            escape(data),
        );
    }
    popup.push_str("</ul></span>");
    format!("<span class='ext-tag'>&lt;EXT&gt;{popup}</span>")
}

/// Format a duration in seconds with the largest unit where the value is >= 1.
fn fmt_seconds(s: f64) -> String {
    if s == 0.0 {
        return "0".to_string();
    }
    let a = s.abs();
    let (val, unit) = if a >= 1.0 {
        (s, " s")
    } else if a >= 1e-3 {
        (s * 1e3, "ms")
    } else if a >= 1e-6 {
        (s * 1e6, "µs")
    } else {
        (s * 1e9, "ns")
    };
    format!("{val:.1} {unit}")
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}
