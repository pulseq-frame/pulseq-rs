//! Shared scaffolding for the two HTML viewers: the outer page shell, plus
//! the small set of helpers (`fmt_seconds`, `json_floats`, `empty_section`)
//! that both viewers used to define independently.

use std::fmt::Write;

use maud::{DOCTYPE, Markup, PreEscaped, html};

const VIEWER_CSS: &str = include_str!("viewer.css");
const VIEWER_JS: &str = include_str!("viewer.js");

/// Wrap a viewer body in the standard page shell. `body_class` is applied to
/// `<body>` so the per-viewer scoped CSS selectors target the right output.
/// `inline_script` is the data-dependent JS for that viewer (shape-data
/// assignments for structured, per-shape Plotly.newPlot calls for raw); it is
/// emitted as a `<script>` block *before* the shared viewer.js so its globals
/// are visible when viewer.js's DOMContentLoaded handler runs.
pub fn page(title: &str, body_class: &str, body: Markup, inline_script: Markup) -> String {
    let doc = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                title { (title) }
                script src="https://cdn.plot.ly/plotly-2.35.2.min.js" charset="utf-8" {}
                style { (PreEscaped(VIEWER_CSS)) }
            }
            body class=(body_class) {
                (body)
                script { (inline_script) }
                script { (PreEscaped(VIEWER_JS)) }
            }
        }
    };
    doc.into_string()
}

pub fn empty_section() -> Markup {
    html! { p.empty { "(not present)" } }
}

/// Format a duration in seconds with the largest unit where the value is >= 1.
pub fn fmt_seconds(s: f64) -> String {
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

pub fn json_floats(xs: &[f64]) -> String {
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
