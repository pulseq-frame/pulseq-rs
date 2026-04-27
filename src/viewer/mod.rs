use std::fmt::Write;
use std::path::Path;

use pulseq_rs::raw::{
    Adc, Block, Delay, ExtensionSpec, Extensions, Gradient, Rf, Section, Shape, Signature, Trap,
    Version,
};

mod util;
use util::*;

const TEMPLATE: &str = include_str!("template.html");

pub fn render(input: &Path, sections: &[Section]) -> String {
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
        let _ = write!(out, "{}: {}", escape(&sig.typ), escape(&sig.hash),);
    }
    out
}

fn render_definitions(defs: &[(String, String)]) -> String {
    render_table(
        "definitions",
        ["key", "value"],
        defs.iter().map(|(k, v)| [escape(k), escape(v)]),
    )
}

fn render_blocks(blocks: &[Block]) -> String {
    render_table(
        "block",
        ["num", "dur", "rf", "gx", "gy", "gz", "adc", "ext"],
        blocks.iter().map(|b| {
            [
                b.id.to_string(),
                render_dur(&b.dur),
                id_ref("rf", b.rf),
                id_ref("grad", b.gx),
                id_ref("grad", b.gy),
                id_ref("grad", b.gz),
                id_ref("adc", b.adc),
                id_ref("ext-ref", b.ext),
            ]
        }),
    )
}

fn render_rfs(rfs: &[Rf]) -> String {
    render_table(
        "rf",
        [
            "id",
            "amp [Hz]",
            "mag",
            "phase",
            "time",
            "delay [s]",
            "freq [Hz]",
            "phase [rad]",
            "shim",
        ],
        rfs.iter().map(|r| {
            let shim = match r.shim_id {
                None => "-".to_string(),
                Some((m, p)) => format!("{}, {}", id_ref("shape", m), id_ref("shape", p)),
            };
            [
                r.id.to_string(),
                r.amp.to_string(),
                id_ref("shape", r.mag_id),
                id_ref("shape", r.phase_id),
                id_ref("shape", r.time_id),
                format!("{:.6}", r.delay),
                r.freq.to_string(),
                format!("{:.4}", r.phase),
                shim,
            ]
        }),
    )
}

fn render_gradients(grads: &[Gradient]) -> String {
    render_table(
        "grad",
        ["id", "amp [Hz/m]", "shape", "time", "delay [s]"],
        grads.iter().map(|g| {
            [
                g.id.to_string(),
                g.amp.to_string(),
                id_ref("shape", g.shape_id),
                id_ref("shape", g.time_id),
                format!("{:.6}", g.delay),
            ]
        }),
    )
}

fn render_traps(traps: &[Trap]) -> String {
    // grad/trap share an ID space — block.gx links to "grad-N" either way.
    render_table(
        "grad",
        [
            "id",
            "amp [Hz/m]",
            "rise [s]",
            "flat [s]",
            "fall [s]",
            "delay [s]",
        ],
        traps.iter().map(|t| {
            [
                t.id.to_string(),
                t.amp.to_string(),
                format!("{:.6}", t.rise),
                format!("{:.6}", t.flat),
                format!("{:.6}", t.fall),
                format!("{:.6}", t.delay),
            ]
        }),
    )
}

fn render_adcs(adcs: &[Adc]) -> String {
    render_table(
        "adc",
        [
            "id",
            "num",
            "dwell [s]",
            "delay [s]",
            "freq [Hz]",
            "phase [rad]",
        ],
        adcs.iter().map(|a| {
            [
                a.id.to_string(),
                a.num.to_string(),
                format!("{:.9}", a.dwell),
                format!("{:.6}", a.delay),
                a.freq.to_string(),
                format!("{:.4}", a.phase),
            ]
        }),
    )
}

fn render_delays(delays: &[Delay]) -> String {
    render_table(
        "delay",
        ["id", "delay [s]"],
        delays
            .iter()
            .map(|d| [d.id.to_string(), format!("{:.6}", d.delay)]),
    )
}

fn render_extensions(ext: &Extensions) -> String {
    let mut s = if ext.refs.is_empty() {
        String::from(r#"<p class="empty">(no references)</p>"#)
    } else {
        render_table(
            "ext-ref",
            ["id", "spec", "obj", "next"],
            ext.refs.iter().map(|r| {
                let obj = if r.obj_id == 0 {
                    "0".to_string()
                } else {
                    format!(
                        r##"<a href="#ext-obj-{}-{}">{}</a>"##,
                        r.spec_id, r.obj_id, r.obj_id
                    )
                };
                [
                    r.id.to_string(),
                    id_ref("ext-spec", r.spec_id),
                    obj,
                    id_ref("ext-ref", r.next),
                ]
            }),
        )
    };

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
    let name = format!("ext-obj-{}", spec.id);
    out.push_str(&render_table(
        &name,
        ["id", "data"],
        spec.instances
            .iter()
            .map(|obj| [obj.id.to_string(), escape(&obj.data)]),
    ));
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
