use std::fmt::Write;
use std::path::Path;

const TEMPLATE: &str = include_str!("template.html");

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
    pub ext_specs: Vec<Table<2>>,
    pub extensions: String,
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
            gradients: Table::new("grad", ["id", "amp [Hz/m]", "shape", "time", "delay [s]"]),
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
            extensions: String::new(),
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
            .replace("__EXTENSIONS__", or_empty(&self.extensions))
            .replace("__SHAPES__", or_empty(&self.shapes))
            .replace("__PLOT_SCRIPTS__", &self.plot_scripts)
    }
}

fn or_empty(content: &str) -> &str {
    if content.is_empty() {
        r#"<p class="empty">(not present)</p>"#
    } else {
        content
    }
}

pub struct Table<const COLUMNS: usize> {
    pub name: &'static str,
    pub column_names: [&'static str; COLUMNS],
    pub rows: Vec<[String; COLUMNS]>,
}

impl<const COLUMNS: usize> Table<COLUMNS> {
    pub fn new(name: &'static str, column_names: [&'static str; COLUMNS]) -> Self {
        Self {
            name,
            column_names,
            rows: Vec::new(),
        }
    }

    #[allow(unused_must_use)]
    pub fn render(&self) -> String {
        if self.rows.is_empty() {
            return "<p class='empty'>(not present)</p>".to_string();
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
