use std::fmt::Write;
use std::path::Path;

const TEMPLATE: &str = include_str!("template.html");
pub const EMPTY_SECTION: &str = "<p class='empty'>(not present)</p>";

pub struct Template {
    pub meta: String,
    pub definitions: Table<2>,
    pub blocks: Table<8>,
    pub rfs: Table<9>,
    pub gradients: Table<5>,
    pub traps: Table<6>,
    pub adcs: Table<6>,
    pub delays: Table<2>,
    pub ext_refs: Table<4>,
    pub ext_specs: Vec<ExtSpec>,
    pub shapes: String,
    pub plot_scripts: String,
}

impl Template {
    pub fn new() -> Self {
        Self {
            meta: String::new(),
            definitions: Table::new("definitions", ["key", "value"]),
            blocks: Table::new(
                "block",
                ["num", "dur", "rf", "gx", "gy", "gz", "adc", "ext"],
            ),
            rfs: Table::new(
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
            ),
            gradients: Table::new(
                "grad",
                ["id", "amp [Hz/m]", "shape", "time", "delay [s]"],
            ),
            traps: Table::new(
                "grad",
                [
                    "id",
                    "amp [Hz/m]",
                    "rise [s]",
                    "flat [s]",
                    "fall [s]",
                    "delay [s]",
                ],
            ),
            adcs: Table::new(
                "adc",
                [
                    "id",
                    "num",
                    "dwell [s]",
                    "delay [s]",
                    "freq [Hz]",
                    "phase [rad]",
                ],
            ),
            delays: Table::new("delay", ["id", "delay [s]"]),
            ext_refs: Table::new("ext-ref", ["id", "spec", "obj", "next"]),
            ext_specs: Vec::new(),
            shapes: String::new(),
            plot_scripts: String::new(),
        }
    }

    pub fn render(&self, file_name: &Path) -> String {
        let ext_specs: String = self
            .ext_specs
            .iter()
            .map(|ext_spec| ext_spec.render())
            .collect();

        TEMPLATE
            .replace(
                "__TITLE__",
                &super::util::escape(&file_name.display().to_string()),
            )
            .replace("__META__", &self.meta)
            .replace("__DEFINITIONS__", &self.definitions.render())
            .replace("__BLOCKS__", &self.blocks.render())
            .replace("__RFS__", &self.rfs.render())
            .replace("__GRADIENTS__", &self.gradients.render())
            .replace("__TRAPS__", &self.traps.render())
            .replace("__ADCS__", &self.adcs.render())
            .replace("__DELAYS__", &self.delays.render())
            .replace("__EXT_REFS__", &self.ext_refs.render())
            .replace("__EXT_SPECS__", &ext_specs)
            .replace("__SHAPES__", &self.shapes)
            .replace("__PLOT_SCRIPTS__", &self.plot_scripts)
    }
}

pub struct Table<const COLUMNS: usize> {
    pub name: String,
    pub column_names: [&'static str; COLUMNS],
    pub rows: Vec<[String; COLUMNS]>,
}

impl<const COLUMNS: usize> Table<COLUMNS> {
    pub fn new(name: impl Into<String>, column_names: [&'static str; COLUMNS]) -> Self {
        Self {
            name: name.into(),
            column_names,
            rows: Vec::new(),
        }
    }

    #[allow(unused_must_use)]
    pub fn render(&self) -> String {
        if self.rows.is_empty() {
            return EMPTY_SECTION.to_string();
        }

        let mut s = String::new();
        write!(
            s,
            "<div class='table-wrap'><table class='{}'><thead><tr>",
            self.name
        );
        for col in self.column_names {
            write!(s, "<th>{col}</th>");
        }
        write!(s, "</tr></thead><tbody>");
        for row in &self.rows {
            write!(s, r#"<tr id="{}-{}">"#, self.name, row[0]);
            for content in row {
                write!(s, "<td>{content}</td>");
            }
            write!(s, "</tr>");
        }
        write!(s, "</tbody></table></div>");

        s
    }
}

pub struct ExtSpec {
    pub spec_id: u32,
    pub name: String,
    pub table: Table<2>,
}

impl ExtSpec {
    pub fn new(spec_id: u32, name: String) -> Self {
        Self {
            spec_id,
            name,
            table: Table::new(format!("ext-obj-{}", spec_id), ["id", "data"]),
        }
    }

    pub fn render(&self) -> String {
        let mut out = format!(
            "<h3 id='ext-spec-{id}'>#{id} {name}</h3>",
            id = self.spec_id,
            name = super::util::escape(&self.name),
        );

        out.push_str(&self.table.render());
        out
    }
}
