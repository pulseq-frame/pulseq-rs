use pulseq_rs::raw::BlockDuration;
use std::fmt::Write;

/// Render a table with sticky headers. The `name` is used both as the table's
/// CSS class and as the prefix for each row's anchor id (`<tr id="{name}-{col0}">`).
/// Tables that aren't actually navigated to still get row ids — harmless,
/// just unused.
#[allow(unused_must_use)]
pub fn render_table<const COLUMNS: usize>(
    name: &str,
    column_names: [&str; COLUMNS],
    rows: impl Iterator<Item = [String; COLUMNS]>,
) -> String {
    let mut s = String::new();
    write!(
        s,
        "<div class='table-wrap'><table class='{name}'><thead><tr>"
    );
    for col in column_names {
        write!(s, "<th>{col}</th>");
    }
    write!(s, "</tr></thead><tbody>");
    for row in rows {
        write!(s, r#"<tr id="{name}-{}">"#, row[0]);
        for content in &row {
            write!(s, "<td>{content}</td>");
        }
        write!(s, "</tr>");
    }
    write!(s, "</tbody></table></div>");

    s
}

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
