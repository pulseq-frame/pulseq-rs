use pulseq_rs::raw::BlockDuration;
use std::fmt::Write;

pub fn render_dur(d: &BlockDuration) -> String {
    match d {
        BlockDuration::Duration(n) => format!("{n}"),
        BlockDuration::DelayId(0) => "0".to_string(),
        BlockDuration::DelayId(n) => format!(r##"<a href="#delay-{n}">#{n}</a>"##),
    }
}

pub fn id_ref(prefix: &str, id: u32) -> String {
    if id == 0 {
        "0".to_string()
    } else {
        format!(r##"<a href="#{prefix}-{id}">{id}</a>"##)
    }
}

pub fn escape(s: &str) -> String {
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
