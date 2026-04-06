pub struct TableDocEntry {
    pub table: Option<&'static str>,
    pub columns: &'static [(&'static str, &'static str)],
}

impl TableDocEntry {
    pub fn column_doc(&self, column_name: &str) -> Option<&'static str> {
        self.columns
            .iter()
            .find(|(name, _)| *name == column_name)
            .map(|(_, desc)| *desc)
    }
}

include!(concat!(env!("OUT_DIR"), "/sqlite_docs_generated.rs"));
