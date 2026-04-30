use std::path::Path;

use maud::{Markup, PreEscaped, html};
use pulseq_rs::raw::{BlockDuration, Section, Shape, Signature, Version};

use crate::viewer::{empty_section, json_floats, page};

pub fn render(input: &Path, sections: &[Section]) -> String {
    let title = input.display().to_string();
    let mut version: Option<&Version> = None;
    let mut signature: Option<&Signature> = None;

    let mut definitions: Vec<(String, [Markup; 2])> = Vec::new();
    let mut blocks: Vec<(String, [Markup; 8])> = Vec::new();
    let mut rfs: Vec<(String, [Markup; 13])> = Vec::new();
    let mut gradients: Vec<(String, [Markup; 5])> = Vec::new();
    let mut traps: Vec<(String, [Markup; 6])> = Vec::new();
    let mut adcs: Vec<(String, [Markup; 9])> = Vec::new();
    let mut delays: Vec<(String, [Markup; 2])> = Vec::new();
    let mut ext_refs: Vec<(String, [Markup; 4])> = Vec::new();
    let mut ext_specs: Vec<ExtSpec> = Vec::new();
    let mut shapes: &[Shape] = &[];

    for section in sections {
        match section {
            Section::Version(v) => version = Some(v),
            Section::Signature(s) => signature = Some(s),
            Section::Definitions(d) => {
                for (k, v) in d {
                    definitions.push((k.clone(), [text(k), text(v)]));
                }
            }
            Section::Blocks(bs) => {
                for b in bs {
                    blocks.push((
                        b.id.to_string(),
                        [
                            text(b.id.to_string()),
                            render_dur(&b.dur),
                            id_ref("rf", b.rf),
                            id_ref("grad", b.gx),
                            id_ref("grad", b.gy),
                            id_ref("grad", b.gz),
                            id_ref("adc", b.adc),
                            id_ref("ext-ref", b.ext),
                        ],
                    ));
                }
            }
            Section::Rfs(rs) => {
                for rf in rs {
                    let shim = match rf.shim_id {
                        None => text("-"),
                        Some((m, p)) => html! {
                            (id_ref("shape", m)) ", " (id_ref("shape", p))
                        },
                    };
                    rfs.push((
                        rf.id.to_string(),
                        [
                            text(rf.id.to_string()),
                            text(rf.amp.to_string()),
                            id_ref("shape", rf.mag_id),
                            id_ref("shape", rf.phase_id),
                            id_ref("shape", rf.time_id),
                            text(rf.center.map_or("-".to_owned(), |x| x.to_string())),
                            text(format!("{:.6}", rf.delay)),
                            text(rf.freq_rel.to_string()),
                            text(rf.phase_rel.to_string()),
                            text(rf.freq_off.to_string()),
                            text(rf.phase_off.to_string()),
                            shim,
                            text(rf.rf_use.to_string()),
                        ],
                    ));
                }
            }
            Section::Gradients(gs) => {
                for g in gs {
                    gradients.push((
                        g.id.to_string(),
                        [
                            text(g.id.to_string()),
                            text(g.amp.to_string()),
                            id_ref("shape", g.shape_id),
                            id_ref("shape", g.time_id),
                            text(format!("{:.6}", g.delay)),
                        ],
                    ));
                }
            }
            Section::Traps(ts) => {
                // grad/trap share an ID space — block.gx links to "grad-N" either way.
                for t in ts {
                    traps.push((
                        t.id.to_string(),
                        [
                            text(t.id.to_string()),
                            text(t.amp.to_string()),
                            text(format!("{:.6}", t.rise)),
                            text(format!("{:.6}", t.flat)),
                            text(format!("{:.6}", t.fall)),
                            text(format!("{:.6}", t.delay)),
                        ],
                    ));
                }
            }
            Section::Adcs(adcs_in) => {
                for a in adcs_in {
                    adcs.push((
                        a.id.to_string(),
                        [
                            text(a.id.to_string()),
                            text(a.num.to_string()),
                            text(format!("{:.9}", a.dwell)),
                            text(format!("{:.6}", a.delay)),
                            text(a.freq_rel.to_string()),
                            text(a.phase_rel.to_string()),
                            text(a.freq_off.to_string()),
                            text(a.phase_off.to_string()),
                            id_ref("shape", a.phase_shape_id),
                        ],
                    ));
                }
            }
            Section::Delays(ds) => {
                for d in ds {
                    delays.push((
                        d.id.to_string(),
                        [text(d.id.to_string()), text(format!("{:.6}", d.delay))],
                    ));
                }
            }
            Section::ExtensionRefs(rs) => {
                for r in rs {
                    let obj = if r.obj_id == 0 {
                        text("0")
                    } else {
                        html! { a href=(format!("#ext-obj-{}-{}", r.spec_id, r.obj_id)) { (r.obj_id) } }
                    };
                    ext_refs.push((
                        r.id.to_string(),
                        [
                            text(r.id.to_string()),
                            id_ref("ext-spec", r.spec_id),
                            obj,
                            id_ref("ext-ref", r.next),
                        ],
                    ));
                }
            }
            Section::ExtensionSpecs(ss) => {
                for spec in ss {
                    let mut rows: Vec<(String, [Markup; 2])> = Vec::new();
                    for obj in &spec.instances {
                        rows.push((
                            obj.id.to_string(),
                            [text(obj.id.to_string()), text(&obj.data)],
                        ));
                    }
                    ext_specs.push(ExtSpec {
                        spec_id: spec.id,
                        name: spec.name.clone(),
                        rows,
                    });
                }
            }
            Section::Shapes(sh) => shapes = sh,
        }
    }

    let body = html! {
        nav {
            a href="#definitions" { "Definitions" }
            a href="#blocks" { "Blocks" }
            a href="#rfs" { "RF" }
            a href="#gradients" { "Gradients" }
            a href="#traps" { "Traps" }
            a href="#adcs" { "ADC" }
            a href="#delays" { "Delays" }
            a href="#extensions" { "Extensions" }
            a href="#shapes" { "Shapes" }
        }

        h1 { (title) }
        p.meta { (render_meta(version.expect("version section (parsing fails without)"), signature)) }

        section id="definitions" { h2 { "Definitions" } (table("definitions", ["key", "value"], &definitions)) }
        section id="blocks" { h2 { "Blocks" }
            (table("block", ["num", "dur", "rf", "gx", "gy", "gz", "adc", "ext"], &blocks))
        }
        section id="rfs" { h2 { "RF events" }
            (table(
                "rf",
                ["id", "amp [Hz]", "mag", "phase", "time", "center [s]", "delay [s]", "freq [rel]", "phase [rel]", "freq [Hz]", "phase [rad]", "shim", "use"],
                &rfs,
            ))
        }
        section id="gradients" { h2 { "Arbitrary gradients" }
            (table("grad", ["id", "amp [Hz/m]", "shape", "time", "delay [s]"], &gradients))
        }
        section id="traps" { h2 { "Trapezoidal gradients" }
            (table(
                "grad",
                ["id", "amp [Hz/m]", "rise [s]", "flat [s]", "fall [s]", "delay [s]"],
                &traps,
            ))
        }
        section id="adcs" { h2 { "ADC events" }
            (table(
                "adc",
                ["id", "num", "dwell [s]", "delay [s]", "freq [rel]", "phase [rel]", "freq [Hz]", "phase [rad]", "phase shape"],
                &adcs,
            ))
        }
        section id="delays" { h2 { "Delays" }
            (table("delay", ["id", "delay [s]"], &delays))
        }
        section id="extensions" { h2 { "Extensions" }
            (table("ext-ref", ["id", "spec", "obj", "next"], &ext_refs))
            @for spec in &ext_specs { (render_ext_spec(spec)) }
        }
        section id="shapes" { h2 { "Shapes" } (render_shapes_section(shapes)) }
    };

    let inline_script = html! {
        "document.addEventListener('DOMContentLoaded', function () {\n"
        (PreEscaped(plot_scripts(shapes)))
        "});\n"
    };

    page(&title, "viewer-raw", body, inline_script)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Wrap a `&str` / `String` as plain auto-escaped Markup for use in cells.
fn text(s: impl AsRef<str>) -> Markup {
    html! { (s.as_ref()) }
}

fn id_ref(prefix: &str, id: u32) -> Markup {
    if id == 0 {
        text("0")
    } else {
        html! { a href=(format!("#{prefix}-{id}")) { (id) } }
    }
}

fn render_dur(d: &BlockDuration) -> Markup {
    match d {
        BlockDuration::Duration(n) => text(n.to_string()),
        BlockDuration::DelayId(0) => text("0"),
        BlockDuration::DelayId(n) => html! {
            a href=(format!("#delay-{n}")) { "#" (n) }
        },
    }
}

fn render_meta(v: &Version, sig: Option<&Signature>) -> Markup {
    html! {
        (format!("pulseq {}.{}.{}{}",
            v.major, v.minor, v.revision, v.rev_suppl.as_deref().unwrap_or("")))
        @if let Some(sig) = sig {
            " · " (sig.typ) ": " (sig.hash)
        }
    }
}

fn table<const N: usize>(name: &str, cols: [&str; N], rows: &[(String, [Markup; N])]) -> Markup {
    if rows.is_empty() {
        return empty_section();
    }
    html! {
        div.table-wrap { table class=(name) {
            thead { tr { @for c in cols { th { (c) } } } }
            tbody {
                @for (id_str, row) in rows {
                    tr id=(format!("{name}-{id_str}")) {
                        @for cell in row { td { (cell) } }
                    }
                }
            }
        } }
    }
}

struct ExtSpec {
    spec_id: u32,
    name: String,
    rows: Vec<(String, [Markup; 2])>,
}

fn render_ext_spec(spec: &ExtSpec) -> Markup {
    html! {
        h3 id=(format!("ext-spec-{}", spec.spec_id)) {
            "#" (spec.spec_id) " " (spec.name)
        }
        (table(
            &format!("ext-obj-{}", spec.spec_id),
            ["id", "data"],
            &spec.rows,
        ))
    }
}

fn render_shapes_section(shapes: &[Shape]) -> Markup {
    if shapes.is_empty() {
        return empty_section();
    }
    html! {
        @for s in shapes {
            h3 id=(format!("shape-{}", s.id)) {
                "Shape #" (s.id) " (" (s.samples.len()) " samples)"
            }
            div.plot id=(format!("shape-plot-{}", s.id)) {}
        }
    }
}

fn plot_scripts(shapes: &[Shape]) -> String {
    let mut out = String::new();
    for s in shapes {
        out.push_str(&format!(
            "Plotly.newPlot('shape-plot-{id}', [{{y: {y}, mode: 'lines', line: {{width: 1.2}}}}], \
             Object.assign({{}}, common, {{xaxis: {{title: 'sample'}}}}), \
             {{responsive: true, displaylogo: false}});\n",
            id = s.id,
            y = json_floats(&s.samples),
        ));
    }
    out
}
