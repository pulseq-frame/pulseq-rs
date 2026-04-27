use std::path::Path;

use pulseq_rs::Sequence;

const TEMPLATE: &str = include_str!("template.html");

pub fn render(input: &Path, seq: &Sequence) -> String {
    let title = escape(&input.display().to_string());
    let meta = "structured sequence view".to_string();

    let body = format!(
        "<p>Name: {}</p>\
         <p>Blocks: {}</p>\
         <p>Definitions: {}</p>",
        seq.name
            .as_deref()
            .map(escape)
            .unwrap_or_else(|| r#"<span class="empty">(unset)</span>"#.to_string()),
        seq.blocks.len(),
        seq.definitions.len(),
    );

    TEMPLATE
        .replace("__TITLE__", &title)
        .replace("__META__", &meta)
        .replace("__BODY__", &body)
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
