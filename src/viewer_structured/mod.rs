use std::fmt::Write;
use std::path::Path;

use pulseq_rs::{Adc, Block, Gradient, Rf, Sequence, Shape};

const TEMPLATE: &str = include_str!("template.html");

pub fn render(input: &Path, seq: &Sequence) -> String {
    let mut plot_scripts = String::new();
    let sequence_html = render_sequence(seq, &mut plot_scripts);

    TEMPLATE
        .replace("__TITLE__", &escape(&input.display().to_string()))
        .replace("__META__", &render_meta(seq))
        .replace("__DEFINITIONS__", &render_definitions(seq))
        .replace("__SEQUENCE__", &sequence_html)
        .replace("/*__PLOT_SCRIPTS__*/", &plot_scripts)
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

fn render_sequence(seq: &Sequence, plot_scripts: &mut String) -> String {
    if seq.blocks.is_empty() {
        return r#"<p class="empty">(no blocks)</p>"#.to_string();
    }

    let mut counter: u32 = 0;
    let mut out = String::from("<table class='sequence'><tbody>");
    for block in &seq.blocks {
        let _ = write!(
            out,
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
            block.id,
            fmt_seconds(block.duration),
            block_events(block, plot_scripts, &mut counter),
        );
    }
    out.push_str("</tbody></table>");
    out
}

fn block_events(block: &Block, plot_scripts: &mut String, counter: &mut u32) -> String {
    let mut tags: Vec<String> = Vec::new();
    if let Some(rf) = &block.rf {
        tags.push(render_rf_tag(rf, plot_scripts, counter));
    }
    for (axis, grad) in [("GX", &block.gx), ("GY", &block.gy), ("GZ", &block.gz)] {
        if let Some(grad) = grad {
            tags.push(render_grad_tag(axis, grad, plot_scripts, counter));
        }
    }
    if let Some(adc) = &block.adc {
        tags.push(render_adc_tag(adc));
    }
    if !block.ext.is_empty() {
        tags.push(render_ext_tag(&block.ext));
    }
    tags.join(" ")
}

fn render_rf_tag(rf: &Rf, plot_scripts: &mut String, counter: &mut u32) -> String {
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
        render_shape_link(&rf.amp_shape, plot_scripts, counter),
    );
    let _ = write!(
        popup,
        "<li><strong>phase shape</strong>{}</li>",
        render_shape_link(&rf.phase_shape, plot_scripts, counter),
    );
    if let Some((mag, phase)) = &rf.shim_shape {
        let _ = write!(
            popup,
            "<li><strong>shim mag</strong>{}</li>",
            render_shape_link(mag, plot_scripts, counter),
        );
        let _ = write!(
            popup,
            "<li><strong>shim phase</strong>{}</li>",
            render_shape_link(phase, plot_scripts, counter),
        );
    }
    popup.push_str("</ul></span>");
    format!("<span class='rf-tag'>&lt;RF&gt;{popup}</span>")
}

fn render_shape_link(shape: &Shape, plot_scripts: &mut String, counter: &mut u32) -> String {
    let id = *counter;
    *counter += 1;
    let _ = writeln!(
        plot_scripts,
        "Plotly.newPlot('shape-plot-{id}', \
          [{{y: {y}, mode: 'lines', line: {{width: 1.2}}}}], \
          Object.assign({{}}, common, {{width: 480, height: 240, xaxis: {{title: 'sample'}}}}), \
          {{responsive: false, displaylogo: false, displayModeBar: false}});",
        y = json_floats(&shape.0),
    );
    format!(
        "<span class='shape-link'>{n} samples\
          <span class='shape-popup'><div id='shape-plot-{id}' class='shape-plot'></div></span>\
        </span>",
        n = shape.0.len(),
    )
}

fn render_grad_tag(
    axis: &str,
    grad: &Gradient,
    plot_scripts: &mut String,
    counter: &mut u32,
) -> String {
    match grad {
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
                render_shape_link(shape, plot_scripts, counter),
            );
            popup.push_str("</ul></span>");
            format!("<span class='free-tag'>&lt;{axis}&gt;{popup}</span>")
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
            format!("<span class='trap-tag'>&lt;{axis}&gt;{popup}</span>")
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

fn render_adc_tag(adc: &Adc) -> String {
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
    format!("<span class='adc-tag'>&lt;ADC&gt;{popup}</span>")
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
