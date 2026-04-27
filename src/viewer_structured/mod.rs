use std::fmt::Write;
use std::path::Path;

use pulseq_rs::{Block, Sequence};

const TEMPLATE: &str = include_str!("template.html");

pub fn render(input: &Path, seq: &Sequence) -> String {
    TEMPLATE
        .replace("__TITLE__", &escape(&input.display().to_string()))
        .replace("__META__", &render_meta(seq))
        .replace("__DEFINITIONS__", &render_definitions(seq))
        .replace("__SEQUENCE__", &render_sequence(seq))
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

fn render_sequence(seq: &Sequence) -> String {
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
            block_events(block),
        );
    }
    out.push_str("</tbody></table>");
    out
}

fn block_events(block: &Block) -> String {
    let mut tags: Vec<String> = Vec::new();
    if block.rf.is_some() {
        tags.push("&lt;RF&gt;".into());
    }
    if block.gx.is_some() {
        tags.push("&lt;GX&gt;".into());
    }
    if block.gy.is_some() {
        tags.push("&lt;GY&gt;".into());
    }
    if block.gz.is_some() {
        tags.push("&lt;GZ&gt;".into());
    }
    if block.adc.is_some() {
        tags.push("&lt;ADC&gt;".into());
    }
    if !block.ext.is_empty() {
        tags.push(render_ext_tag(&block.ext));
    }
    tags.join(" ")
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
