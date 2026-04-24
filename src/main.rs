use std::env;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use pulseq_rs::raw::{
    Adc, Block, BlockDuration, Delay, ExtensionObject, ExtensionSpec, Extensions, Gradient, Rf,
    Section, Shape, Signature, Trap, Version,
};

const TEMPLATE: &str = include_str!("template.html");

fn main() -> ExitCode {
    let mut args = env::args();
    let prog = args.next().unwrap_or_else(|| "pulseq-rs".into());
    let Some(path) = args.next() else {
        eprintln!("usage: {prog} <seq-file>");
        return ExitCode::from(2);
    };
    let input = PathBuf::from(path);
    let source = match fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("read error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let sections = match pulseq_rs::parse_file(&source) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("parse error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let html = render(&input, &sections);
    let out = input.with_extension("html");
    if let Err(e) = fs::write(&out, html) {
        eprintln!("write error: {e}");
        return ExitCode::FAILURE;
    }
    println!("wrote {}", out.display());
    ExitCode::SUCCESS
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
            Section::Traps(t) => {
                let (html, js) = render_traps(t);
                s.traps = html;
                s.plot_scripts.push_str(&js);
            }
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
        .replace("{{TITLE}}", &escape(&input.display().to_string()))
        .replace("{{META}}", &s.meta)
        .replace("{{DEFINITIONS}}", or_empty(&s.definitions))
        .replace("{{BLOCKS}}", or_empty(&s.blocks))
        .replace("{{RFS}}", or_empty(&s.rfs))
        .replace("{{GRADIENTS}}", or_empty(&s.gradients))
        .replace("{{TRAPS}}", or_empty(&s.traps))
        .replace("{{ADCS}}", or_empty(&s.adcs))
        .replace("{{DELAYS}}", or_empty(&s.delays))
        .replace("{{EXTENSIONS}}", or_empty(&s.extensions))
        .replace("{{SHAPES}}", or_empty(&s.shapes))
        .replace("{{PLOT_SCRIPTS}}", &s.plot_scripts)
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
    let mut s = String::from(r#"<dl class="definitions">"#);
    for (k, v) in defs {
        let _ = write!(s, "<dt>{}</dt><dd>{}</dd>", escape(k), escape(v));
    }
    s.push_str("</dl>");
    s
}

fn render_blocks(blocks: &[Block]) -> String {
    let mut s = String::from(
        "<table><thead><tr>\
         <th>#</th><th>dur</th><th>rf</th><th>gx</th><th>gy</th><th>gz</th>\
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
    s.push_str("</tbody></table>");
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
        "<table><thead><tr>\
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
    s.push_str("</tbody></table>");
    s
}

fn render_gradients(grads: &[Gradient]) -> String {
    let mut s = String::from(
        "<table><thead><tr>\
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
    s.push_str("</tbody></table>");
    s
}

fn render_traps(traps: &[Trap]) -> (String, String) {
    let mut s = String::from(
        "<table><thead><tr>\
         <th>id</th><th>amp [Hz/m]</th><th>rise [s]</th><th>flat [s]</th><th>fall [s]</th>\
         <th>delay [s]</th><th>plot</th></tr></thead><tbody>",
    );
    let mut js = String::new();
    for t in traps {
        let _ = write!(s, r#"<tr id="grad-{}">"#, t.id);
        let _ = write!(s, "<td>{}</td>", t.id);
        let _ = write!(s, "<td>{}</td>", t.amp);
        let _ = write!(s, "<td>{:.6}</td>", t.rise);
        let _ = write!(s, "<td>{:.6}</td>", t.flat);
        let _ = write!(s, "<td>{:.6}</td>", t.fall);
        let _ = write!(s, "<td>{:.6}</td>", t.delay);
        let _ = write!(
            s,
            r#"<td style="padding:0"><div class="plot trap" id="trap-plot-{}"></div></td>"#,
            t.id
        );
        s.push_str("</tr>");
        // Trapezoid waveform: flat at 0 during delay, rises linearly, plateau, falls back.
        let t0 = 0.0;
        let t1 = t.delay;
        let t2 = t.delay + t.rise;
        let t3 = t.delay + t.rise + t.flat;
        let t4 = t.delay + t.rise + t.flat + t.fall;
        let _ = writeln!(
            js,
            "Plotly.newPlot('trap-plot-{id}', [{{x: [{t0},{t1},{t2},{t3},{t4}], \
              y: [0,0,{amp},{amp},0], mode: 'lines', line: {{width: 1.5}}}}], \
              Object.assign({{}}, common, {{margin: {{t: 4, r: 4, b: 22, l: 40}}}}), \
              {{responsive: true, displayModeBar: false}});",
            id = t.id,
            amp = t.amp,
            t0 = t0,
            t1 = t1,
            t2 = t2,
            t3 = t3,
            t4 = t4,
        );
    }
    s.push_str("</tbody></table>");
    (s, js)
}

fn render_adcs(adcs: &[Adc]) -> String {
    let mut s = String::from(
        "<table><thead><tr>\
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
    s.push_str("</tbody></table>");
    s
}

fn render_delays(delays: &[Delay]) -> String {
    let mut s = String::from(
        "<table><thead><tr><th>id</th><th>delay [s]</th></tr></thead><tbody>",
    );
    for d in delays {
        let _ = write!(s, r#"<tr id="delay-{}">"#, d.id);
        let _ = write!(s, "<td>{}</td>", d.id);
        let _ = write!(s, "<td>{:.6}</td>", d.delay);
        s.push_str("</tr>");
    }
    s.push_str("</tbody></table>");
    s
}

fn render_extensions(ext: &Extensions) -> String {
    let mut s = String::new();

    s.push_str("<h3>References</h3>");
    if ext.refs.is_empty() {
        s.push_str(r#"<p class="empty">(none)</p>"#);
    } else {
        s.push_str(
            "<table><thead><tr>\
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
        s.push_str("</tbody></table>");
    }

    s.push_str("<h3>Specifications</h3>");
    if ext.specs.is_empty() {
        s.push_str(r#"<p class="empty">(none)</p>"#);
    } else {
        for spec in &ext.specs {
            render_ext_spec(&mut s, spec);
        }
    }
    s
}

fn render_ext_spec(out: &mut String, spec: &ExtensionSpec) {
    let _ = write!(
        out,
        r#"<div id="ext-spec-{id}" class="shape-block"><h3>#{id} {name}</h3>"#,
        id = spec.id,
        name = escape(&spec.name),
    );
    if spec.instances.is_empty() {
        out.push_str(r#"<p class="empty">(no instances)</p>"#);
    } else {
        out.push_str(r#"<ul class="ext-objs">"#);
        for obj in &spec.instances {
            render_ext_obj(out, spec.id, obj);
        }
        out.push_str("</ul>");
    }
    out.push_str("</div>");
}

fn render_ext_obj(out: &mut String, spec_id: u32, obj: &ExtensionObject) {
    let _ = write!(
        out,
        r#"<li id="ext-obj-{spec_id}-{id}"><strong>#{id}</strong> {data}</li>"#,
        spec_id = spec_id,
        id = obj.id,
        data = escape(&obj.data),
    );
}

fn render_shapes(shapes: &[Shape]) -> (String, String) {
    let mut html = String::new();
    let mut js = String::new();
    for shape in shapes {
        let _ = write!(
            html,
            r#"<div class="shape-block"><h3 id="shape-{id}">Shape #{id} ({n} samples)</h3><div class="plot" id="shape-plot-{id}"></div></div>"#,
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
