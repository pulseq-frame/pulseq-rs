use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;

use pulseq_rs::raw::{
    Adc, Block, BlockDuration, Delay, ExtensionSpec, Extensions, Gradient, Rf, Section, Shape,
    Signature, Trap, Version,
};

const TEMPLATE: &str = include_str!("template.html");

/// Parse a pulseq .seq file and render it as a standalone HTML viewer.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to the input .seq file.
    input: PathBuf,

    /// Write the rendered HTML to this path. Defaults to a temporary file.
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Open the rendered HTML in the default browser.
    /// Implied when no --output is given.
    #[arg(long)]
    open: bool,
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();

    // Resolve output path + default for --open.
    let (output, open) = match cli.output {
        Some(path) => (path, cli.open),
        None => (default_output_path(&cli.input), true),
    };

    let source = fs::read_to_string(&cli.input)
        .map_err(|e| format!("failed to read {}: {e}", cli.input.display()))?;

    let sections = pulseq_rs::parse_file(&source).map_err(|e| format!("parse error: {e}"))?;

    let html = render(&cli.input, &sections);

    fs::write(&output, html).map_err(|e| format!("failed to write {}: {e}", output.display()))?;

    println!("wrote {}", output.display());

    if open {
        open::that(&output).map_err(|e| format!("failed to open {}: {e}", output.display()))?;
    }

    Ok(())
}

fn default_output_path(input: &Path) -> PathBuf {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "pulseq".into());
    std::env::temp_dir().join(format!("{stem}-{}.html", std::process::id()))
}

// ---------------------------------------------------------------------------
// Top-level render
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Slots {
    meta: String,
    definitions: String,
    blocks: String,
    rfs: String,
    gradients: String,
    traps: String,
    adcs: String,
    delays: String,
    extensions: String,
    shapes: String,
    plot_scripts: String,
}

fn render(input: &Path, sections: &[Section]) -> String {
    let mut s = Slots::default();
    let mut version: Option<&Version> = None;
    let mut signature: Option<&Signature> = None;

    for section in sections {
        match section {
            Section::Version(v) => version = Some(v),
            Section::Signature(sig) => signature = Some(sig),
            Section::Definitions(d) => s.definitions = render_definitions(d),
            Section::Blocks(b) => s.blocks = render_blocks(b),
            Section::Rfs(r) => s.rfs = render_rfs(r),
            Section::Gradients(g) => s.gradients = render_gradients(g),
            Section::Traps(t) => s.traps = render_traps(t),
            Section::Adcs(a) => s.adcs = render_adcs(a),
            Section::Delays(d) => s.delays = render_delays(d),
            Section::Extensions(e) => s.extensions = render_extensions(e),
            Section::Shapes(sh) => {
                let (html, js) = render_shapes(sh);
                s.shapes = html;
                s.plot_scripts.push_str(&js);
            }
        }
    }

    s.meta = render_meta(version, signature);

    TEMPLATE
        .replace("__TITLE__", &escape(&input.display().to_string()))
        .replace("__META__", &s.meta)
        .replace("__DEFINITIONS__", or_empty(&s.definitions))
        .replace("__BLOCKS__", or_empty(&s.blocks))
        .replace("__RFS__", or_empty(&s.rfs))
        .replace("__GRADIENTS__", or_empty(&s.gradients))
        .replace("__TRAPS__", or_empty(&s.traps))
        .replace("__ADCS__", or_empty(&s.adcs))
        .replace("__DELAYS__", or_empty(&s.delays))
        .replace("__EXTENSIONS__", or_empty(&s.extensions))
        .replace("__SHAPES__", or_empty(&s.shapes))
        .replace("__PLOT_SCRIPTS__", &s.plot_scripts)
}

fn or_empty(content: &str) -> &str {
    if content.is_empty() {
        r#"<p class="empty">(not present)</p>"#
    } else {
        content
    }
}

// ---------------------------------------------------------------------------
// Section renderers
// ---------------------------------------------------------------------------

fn render_meta(v: Option<&Version>, sig: Option<&Signature>) -> String {
    let mut out = String::new();
    if let Some(v) = v {
        let _ = write!(
            out,
            "pulseq {}.{}.{}{}",
            v.major,
            v.minor,
            v.revision,
            v.rev_suppl.as_deref().unwrap_or("")
        );
    }
    if let Some(sig) = sig {
        if !out.is_empty() {
            out.push_str(" &middot; ");
        }
        let _ = write!(
            out,
            "{}: {}",
            escape(&sig.typ),
            escape(&sig.hash),
        );
    }
    out
}

fn render_definitions(defs: &[(String, String)]) -> String {
    let mut s = String::from(
        "<div class=\"table-wrap\"><table class=\"definitions\"><thead><tr>\
         <th>key</th><th>value</th></tr></thead><tbody>",
    );
    for (k, v) in defs {
        let _ = write!(s, "<tr><td>{}</td><td>{}</td></tr>", escape(k), escape(v));
    }
    s.push_str("</tbody></table></div>");
    s
}

fn render_blocks(blocks: &[Block]) -> String {
    let mut s = String::from(
        "<div class=\"table-wrap\"><table><thead><tr>\
         <th>num</th><th>dur</th><th>rf</th><th>gx</th><th>gy</th><th>gz</th>\
         <th>adc</th><th>ext</th></tr></thead><tbody>",
    );
    for b in blocks {
        let _ = write!(s, r#"<tr id="block-{}">"#, b.id);
        let _ = write!(s, "<td>{}</td>", b.id);
        let _ = write!(s, "<td>{}</td>", render_dur(&b.dur));
        let _ = write!(s, "<td>{}</td>", id_ref("rf", b.rf));
        let _ = write!(s, "<td>{}</td>", id_ref("grad", b.gx));
        let _ = write!(s, "<td>{}</td>", id_ref("grad", b.gy));
        let _ = write!(s, "<td>{}</td>", id_ref("grad", b.gz));
        let _ = write!(s, "<td>{}</td>", id_ref("adc", b.adc));
        let _ = write!(s, "<td>{}</td>", id_ref("ext-ref", b.ext));
        s.push_str("</tr>");
    }
    s.push_str("</tbody></table></div>");
    s
}

fn render_dur(d: &BlockDuration) -> String {
    match d {
        BlockDuration::Duration(n) => format!("{n}"),
        BlockDuration::DelayId(0) => "0".to_string(),
        BlockDuration::DelayId(n) => format!(r##"<a href="#delay-{n}">#{n}</a>"##),
    }
}

fn render_rfs(rfs: &[Rf]) -> String {
    let mut s = String::from(
        "<div class=\"table-wrap\"><table><thead><tr>\
         <th>id</th><th>amp [Hz]</th><th>mag</th><th>phase</th><th>time</th>\
         <th>delay [s]</th><th>freq [Hz]</th><th>phase [rad]</th><th>shim</th>\
         </tr></thead><tbody>",
    );
    for r in rfs {
        let shim = match r.shim_id {
            None => "-".to_string(),
            Some((m, p)) => format!("{}, {}", id_ref("shape", m), id_ref("shape", p)),
        };
        let _ = write!(s, r#"<tr id="rf-{}">"#, r.id);
        let _ = write!(s, "<td>{}</td>", r.id);
        let _ = write!(s, "<td>{}</td>", r.amp);
        let _ = write!(s, "<td>{}</td>", id_ref("shape", r.mag_id));
        let _ = write!(s, "<td>{}</td>", id_ref("shape", r.phase_id));
        let _ = write!(s, "<td>{}</td>", id_ref("shape", r.time_id));
        let _ = write!(s, "<td>{:.6}</td>", r.delay);
        let _ = write!(s, "<td>{}</td>", r.freq);
        let _ = write!(s, "<td>{:.4}</td>", r.phase);
        let _ = write!(s, "<td>{shim}</td>");
        s.push_str("</tr>");
    }
    s.push_str("</tbody></table></div>");
    s
}

fn render_gradients(grads: &[Gradient]) -> String {
    let mut s = String::from(
        "<div class=\"table-wrap\"><table><thead><tr>\
         <th>id</th><th>amp [Hz/m]</th><th>shape</th><th>time</th><th>delay [s]</th>\
         </tr></thead><tbody>",
    );
    for g in grads {
        let _ = write!(s, r#"<tr id="grad-{}">"#, g.id);
        let _ = write!(s, "<td>{}</td>", g.id);
        let _ = write!(s, "<td>{}</td>", g.amp);
        let _ = write!(s, "<td>{}</td>", id_ref("shape", g.shape_id));
        let _ = write!(s, "<td>{}</td>", id_ref("shape", g.time_id));
        let _ = write!(s, "<td>{:.6}</td>", g.delay);
        s.push_str("</tr>");
    }
    s.push_str("</tbody></table></div>");
    s
}

fn render_traps(traps: &[Trap]) -> String {
    let mut s = String::from(
        "<div class=\"table-wrap\"><table><thead><tr>\
         <th>id</th><th>amp [Hz/m]</th><th>rise [s]</th><th>flat [s]</th><th>fall [s]</th>\
         <th>delay [s]</th></tr></thead><tbody>",
    );
    for t in traps {
        let _ = write!(s, r#"<tr id="grad-{}">"#, t.id);
        let _ = write!(s, "<td>{}</td>", t.id);
        let _ = write!(s, "<td>{}</td>", t.amp);
        let _ = write!(s, "<td>{:.6}</td>", t.rise);
        let _ = write!(s, "<td>{:.6}</td>", t.flat);
        let _ = write!(s, "<td>{:.6}</td>", t.fall);
        let _ = write!(s, "<td>{:.6}</td>", t.delay);
        s.push_str("</tr>");
    }
    s.push_str("</tbody></table></div>");
    s
}

fn render_adcs(adcs: &[Adc]) -> String {
    let mut s = String::from(
        "<div class=\"table-wrap\"><table><thead><tr>\
         <th>id</th><th>num</th><th>dwell [s]</th><th>delay [s]</th>\
         <th>freq [Hz]</th><th>phase [rad]</th></tr></thead><tbody>",
    );
    for a in adcs {
        let _ = write!(s, r#"<tr id="adc-{}">"#, a.id);
        let _ = write!(s, "<td>{}</td>", a.id);
        let _ = write!(s, "<td>{}</td>", a.num);
        let _ = write!(s, "<td>{:.9}</td>", a.dwell);
        let _ = write!(s, "<td>{:.6}</td>", a.delay);
        let _ = write!(s, "<td>{}</td>", a.freq);
        let _ = write!(s, "<td>{:.4}</td>", a.phase);
        s.push_str("</tr>");
    }
    s.push_str("</tbody></table></div>");
    s
}

fn render_delays(delays: &[Delay]) -> String {
    let mut s = String::from(
        "<div class=\"table-wrap\"><table><thead><tr>\
         <th>id</th><th>delay [s]</th></tr></thead><tbody>",
    );
    for d in delays {
        let _ = write!(s, r#"<tr id="delay-{}">"#, d.id);
        let _ = write!(s, "<td>{}</td>", d.id);
        let _ = write!(s, "<td>{:.6}</td>", d.delay);
        s.push_str("</tr>");
    }
    s.push_str("</tbody></table></div>");
    s
}

fn render_extensions(ext: &Extensions) -> String {
    let mut s = String::new();

    if ext.refs.is_empty() {
        s.push_str(r#"<p class="empty">(no references)</p>"#);
    } else {
        s.push_str(
            "<div class=\"table-wrap\"><table><thead><tr>\
             <th>id</th><th>spec</th><th>obj</th><th>next</th></tr></thead><tbody>",
        );
        for r in &ext.refs {
            let obj = if r.obj_id == 0 {
                "0".to_string()
            } else {
                format!(
                    r##"<a href="#ext-obj-{}-{}">{}</a>"##,
                    r.spec_id, r.obj_id, r.obj_id
                )
            };
            let _ = write!(s, r#"<tr id="ext-ref-{}">"#, r.id);
            let _ = write!(s, "<td>{}</td>", r.id);
            let _ = write!(s, "<td>{}</td>", id_ref("ext-spec", r.spec_id));
            let _ = write!(s, "<td>{obj}</td>");
            let _ = write!(s, "<td>{}</td>", id_ref("ext-ref", r.next));
            s.push_str("</tr>");
        }
        s.push_str("</tbody></table></div>");
    }

    for spec in &ext.specs {
        render_ext_spec(&mut s, spec);
    }
    s
}

fn render_ext_spec(out: &mut String, spec: &ExtensionSpec) {
    let _ = write!(
        out,
        r#"<h3 id="ext-spec-{id}">#{id} {name}</h3>"#,
        id = spec.id,
        name = escape(&spec.name),
    );
    if spec.instances.is_empty() {
        out.push_str(r#"<p class="empty">(no instances)</p>"#);
        return;
    }
    out.push_str(
        "<div class=\"table-wrap\"><table><thead><tr>\
         <th>id</th><th>data</th></tr></thead><tbody>",
    );
    for obj in &spec.instances {
        let _ = write!(
            out,
            r#"<tr id="ext-obj-{spec_id}-{id}"><td>{id}</td><td>{data}</td></tr>"#,
            spec_id = spec.id,
            id = obj.id,
            data = escape(&obj.data),
        );
    }
    out.push_str("</tbody></table></div>");
}

fn render_shapes(shapes: &[Shape]) -> (String, String) {
    let mut html = String::new();
    let mut js = String::new();
    for shape in shapes {
        let _ = write!(
            html,
            r#"<h3 id="shape-{id}">Shape #{id} ({n} samples)</h3><div class="plot" id="shape-plot-{id}"></div>"#,
            id = shape.id,
            n = shape.samples.len(),
        );
        let _ = writeln!(
            js,
            "Plotly.newPlot('shape-plot-{id}', [{{y: {y}, mode: 'lines', \
              line: {{width: 1.2}}}}], \
              Object.assign({{}}, common, {{xaxis: {{title: 'sample'}}}}), \
              {{responsive: true, displaylogo: false}});",
            id = shape.id,
            y = json_floats(&shape.samples),
        );
    }
    (html, js)
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn id_ref(prefix: &str, id: u32) -> String {
    if id == 0 {
        "0".to_string()
    } else {
        format!(r##"<a href="#{prefix}-{id}">{id}</a>"##)
    }
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
