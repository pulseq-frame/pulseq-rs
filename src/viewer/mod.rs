use std::fmt::Write;
use std::path::Path;

use pulseq_rs::raw::{
    Adc, Block, Delay, ExtensionSpec, Extensions, Gradient, Rf, Section, Shape, Signature, Trap,
    Version,
};

mod template;
mod util;
use util::*;

use crate::viewer::template::{Table, Template};

pub fn render(input: &Path, sections: &[Section]) -> String {
    let tmpl = &mut template::Template::new();
    let mut version: Option<&Version> = None;
    let mut signature: Option<&Signature> = None;

    for section in sections {
        match section {
            Section::Version(v) => version = Some(v),
            Section::Signature(sig) => signature = Some(sig),
            Section::Definitions(d) => populate_definitions(tmpl, d),
            Section::Blocks(b) => populate_blocks(tmpl, b),
            Section::Rfs(r) => populate_rfs(tmpl, r),
            Section::Gradients(g) => populate_gradients(tmpl, g),
            Section::Traps(t) => populate_traps(tmpl, t),
            Section::Adcs(a) => populate_adcs(tmpl, a),
            Section::Delays(d) => populate_delays(tmpl, d),
            Section::Extensions(e) => tmpl.extensions = populate_extensions(tmpl, e),
            Section::Shapes(sh) => {
                let (html, js) = render_shapes(sh);
                tmpl.shapes = html;
                tmpl.plot_scripts.push_str(&js);
            }
        }
    }
    tmpl.meta = render_meta(
        version.expect("version section (parsing fails without)"),
        signature,
    );

    tmpl.render(input)
}

// ---------------------------------------------------------------------------
// Section renderers
// ---------------------------------------------------------------------------

fn render_meta(v: &Version, sig: Option<&Signature>) -> String {
    let mut out = format!(
        "pulseq {}.{}.{}{}",
        v.major,
        v.minor,
        v.revision,
        v.rev_suppl.as_deref().unwrap_or("")
    );

    if let Some(sig) = sig {
        if !out.is_empty() {
            out.push_str(" &middot; ");
        }
        let _ = write!(out, "{}: {}", escape(&sig.typ), escape(&sig.hash),);
    }
    out
}

fn populate_definitions(template: &mut Template, defs: &[(String, String)]) {
    for def in defs {
        template
            .definitions
            .rows
            .push([escape(&def.0), escape(&def.1)]);
    }
}

fn populate_blocks(template: &mut Template, blocks: &[Block]) {
    for block in blocks {
        template.blocks.rows.push([
            block.id.to_string(),
            render_dur(&block.dur),
            id_ref("rf", block.rf),
            id_ref("grad", block.gx),
            id_ref("grad", block.gy),
            id_ref("grad", block.gz),
            id_ref("adc", block.adc),
            id_ref("ext-ref", block.ext),
        ])
    }
}

pub fn populate_rfs(template: &mut Template, rfs: &[Rf]) {
    for rf in rfs {
        let shim = match rf.shim_id {
            None => "-".to_string(),
            Some((m, p)) => format!("{}, {}", id_ref("shape", m), id_ref("shape", p)),
        };

        template.rfs.rows.push([
            rf.id.to_string(),
            rf.amp.to_string(),
            id_ref("shape", rf.mag_id),
            id_ref("shape", rf.phase_id),
            id_ref("shape", rf.time_id),
            format!("{:.6}", rf.delay),
            rf.freq.to_string(),
            format!("{:.4}", rf.phase),
            shim,
        ]);
    }
}

fn populate_gradients(template: &mut Template, grads: &[Gradient]) {
    for grad in grads {
        template.gradients.rows.push([
            grad.id.to_string(),
            grad.amp.to_string(),
            id_ref("shape", grad.shape_id),
            id_ref("shape", grad.time_id),
            format!("{:.6}", grad.delay),
        ])
    }
}

fn populate_traps(template: &mut Template, traps: &[Trap]) {
    // grad/trap share an ID space — block.gx links to "grad-N" either way.
    for trap in traps {
        template.traps.rows.push([
            trap.id.to_string(),
            trap.amp.to_string(),
            format!("{:.6}", trap.rise),
            format!("{:.6}", trap.flat),
            format!("{:.6}", trap.fall),
            format!("{:.6}", trap.delay),
        ])
    }
}

fn populate_adcs(template: &mut Template, adcs: &[Adc]) {
    for adc in adcs {
        template.adcs.rows.push([
            adc.id.to_string(),
            adc.num.to_string(),
            format!("{:.9}", adc.dwell),
            format!("{:.6}", adc.delay),
            adc.freq.to_string(),
            format!("{:.4}", adc.phase),
        ]);
    }
}

fn populate_delays(template: &mut Template, delays: &[Delay]) {
    for d in delays {
        template
            .delays
            .rows
            .push([d.id.to_string(), format!("{:.6}", d.delay)]);
    }
}

fn populate_extensions(template: &mut Template, ext: &Extensions) -> String {
    for ext_ref in &ext.refs {
        let obj = if ext_ref.obj_id == 0 {
            "0".to_string()
        } else {
            format!(
                r##"<a href="#ext-obj-{}-{}">{}</a>"##,
                ext_ref.spec_id, ext_ref.obj_id, ext_ref.obj_id
            )
        };
        template.ext_refs.rows.push([
            ext_ref.id.to_string(),
            id_ref("ext-spec", ext_ref.spec_id),
            obj,
            id_ref("ext-ref", ext_ref.next),
        ]);
    }

    let mut s = String::new();
    for spec in &ext.specs {
        populate_ext_spec(&mut s, spec);
    }
    s
}

fn populate_ext_spec(out: &mut String, spec: &ExtensionSpec) {
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
