use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;
use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=sqlite-docs");

    let sqlite_docs_dir = Path::new("sqlite-docs");
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("sqlite_docs_generated.rs");

    let mut code = String::new();

    writeln!(
        code,
        "pub fn table_docs(suffix: &str) -> Option<crate::commands::export::sqlite::sqlite_docs::TableDocEntry> {{"
    )
    .unwrap();
    writeln!(code, "    match suffix {{").unwrap();

    if sqlite_docs_dir.exists() {
        for entry in fs::read_dir(sqlite_docs_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
                continue;
            }
            let suffix = path.file_stem().unwrap().to_str().unwrap();
            let contents = fs::read_to_string(&path).unwrap();
            let doc: YamlDoc = serde_yaml::from_str(&contents)
                .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", path.display()));

            writeln!(
                code,
                "        {suffix:?} => Some(crate::commands::export::sqlite::sqlite_docs::TableDocEntry {{"
            )
            .unwrap();

            match &doc.table {
                Some(t) => {
                    let t = t.trim();
                    writeln!(code, "            table: Some({t:?}),").unwrap();
                }
                None => {
                    writeln!(code, "            table: None,").unwrap();
                }
            }

            writeln!(code, "            columns: &[").unwrap();
            if let Some(columns) = &doc.columns {
                for (col_name, col_desc) in columns {
                    let col_desc = col_desc.trim();
                    writeln!(code, "                ({col_name:?}, {col_desc:?}),").unwrap();
                }
            }
            writeln!(code, "            ],").unwrap();

            writeln!(code, "        }}),").unwrap();
        }
    }

    writeln!(code, "        _ => None,").unwrap();
    writeln!(code, "    }}").unwrap();
    writeln!(code, "}}").unwrap();

    fs::write(&dest_path, code).unwrap();
}

#[derive(serde::Deserialize)]
struct YamlDoc {
    table: Option<String>,
    columns: Option<HashMap<String, String>>,
}
