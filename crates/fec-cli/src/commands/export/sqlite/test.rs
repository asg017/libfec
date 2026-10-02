#[cfg(test)]
mod tests {
    use fec_parser::Filing;

    use super::super::{export_itemizations, init, insert_filing_metadata};

    /// Macro to easily load test filings by ID
    macro_rules! filing {
        ($id:expr) => {{
            let bytes: &[u8] =
                include_bytes!(concat!("../../../../../../.test-files/", $id, ".fec"));
            Filing::from_reader(bytes, $id.to_string(), bytes.len()).unwrap()
        }};
    }
    #[test]
    fn test_basic() {
        assert_eq!(1 + 1, 2);
    }

    fn query(db: &rusqlite::Connection, query: &str) -> String {
        let mut out = String::new();
        let mut stmt = db.prepare(query).unwrap();
        let columns = stmt
            .column_names()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        out.push_str(&columns.join(" | "));
        out.push('\n');
        let mut rows = stmt.query([]).unwrap();
        let mut idx = 0;
        while let Some(row) = rows.next().unwrap() {
            let col_count = row.as_ref().column_count();
            out.push_str(format!("==== Row {} ====\n", idx).as_str());
            idx += 1;

            for (i, col_name) in columns.iter().enumerate().take(col_count) {
                let value: rusqlite::types::Value = row.get(i).unwrap();
                out.push_str(&format!("[{}]:", col_name));
                match value {
                    rusqlite::types::Value::Null => out.push_str("NULL"),
                    rusqlite::types::Value::Integer(i) => out.push_str(&i.to_string()),
                    rusqlite::types::Value::Real(f) => out.push_str(&f.to_string()),
                    rusqlite::types::Value::Text(s) => out.push_str(&s),
                    rusqlite::types::Value::Blob(b) => out.push_str(&format!("{:?}", b)),
                }
                out.push('\n');
            }
        }
        out
    }

    #[test]
    #[ignore]
    fn test_basic_f99() {
        let f99_84 = filing!("1913493");
        let mut db = rusqlite::Connection::open_in_memory().unwrap();
        let mut tx = db.transaction().unwrap();
        init(&mut tx).unwrap();
        insert_filing_metadata(&mut tx, &f99_84).unwrap();
        tx.commit().unwrap();

        insta::assert_snapshot!(query(
            &db,
            "select name, sql from sqlite_master where type='table'"
        ),);
        insta::assert_snapshot!(query(&db, "select count(*) from libfec_f99"),);
        insta::assert_snapshot!(query(&db, "select * from libfec_filings"),);
    }
    #[test]
    #[ignore]
    fn test_f99_84_and_85() {
        let f99_84 = filing!("1913493");
        let f99_85 = filing!("1913595");

        let mut db = rusqlite::Connection::open_in_memory().unwrap();
        let mut tx = db.transaction().unwrap();

        init(&mut tx).unwrap();

        insert_filing_metadata(&mut tx, &f99_84).unwrap();
        tx.commit().unwrap();

        insta::assert_snapshot!("8.4 1st", query(&db, "select * from libfec_f99"),);

        let mut tx = db.transaction().unwrap();
        insert_filing_metadata(&mut tx, &f99_85).unwrap();
        tx.commit().unwrap();
        insta::assert_snapshot!("8.5 2nd", query(&db, "select * from libfec_f99"),);
    }

    #[test]
    fn test_f1s_84_and_85() -> anyhow::Result<()> {
        let f1s_84 = filing!("1913562");
        let f1s_85 = filing!("1923816");
        let mut db = rusqlite::Connection::open_in_memory()?;
        let mut tx = db.transaction()?;
        init(&mut tx)?;
        tx.commit()?;

        let mut tx = db.transaction()?;
        insert_filing_metadata(&mut tx, &f1s_84).unwrap();
        export_itemizations(&mut tx, f1s_84, None, None)?;
        tx.commit()?;

        insta::assert_snapshot!(
            "F1S schema",
            query(
                &db,
                "select sql from sqlite_master where name = 'libfec_F1S'"
            ),
        );
        insta::assert_snapshot!("F1S 8.4 data", query(&db, "select * from libfec_F1S"),);

        let mut tx = db.transaction()?;
        insert_filing_metadata(&mut tx, &f1s_85).unwrap();
        export_itemizations(&mut tx, f1s_85, None, None)?;
        tx.commit()?;
        insta::assert_snapshot!(
            "F1S 8.5 data",
            query(&db, "select * from libfec_F1S where filing_id = '1923816'"),
        );

        Ok(())
    }

    /// A database created before header_style/batch_number/received_date
    /// gets them added, and inserts work.
    #[test]
    fn test_init_adds_columns_to_old_filings_table() -> anyhow::Result<()> {
        let mut db = rusqlite::Connection::open_in_memory()?;
        db.execute(
            "CREATE TABLE libfec_filings(filing_id TEXT PRIMARY KEY NOT NULL, \
             fec_version TEXT NOT NULL, software_name TEXT NOT NULL, \
             software_version TEXT NOT NULL, report_id TEXT, report_number TEXT, \
             comment TEXT, cover_record_form TEXT NOT NULL, \
             cover_record_form_amendment_indicator TEXT, filer_id TEXT NOT NULL, \
             filer_name TEXT NOT NULL, report_code TEXT, coverage_from_date TEXT, \
             coverage_through_date TEXT)",
            [],
        )?;
        let mut tx = db.transaction()?;
        init(&mut tx)?;
        // Idempotent.
        init(&mut tx)?;
        insert_filing_metadata(&mut tx, &filing!("1913493"))?;
        tx.commit()?;
        let style: String =
            db.query_row("select header_style from libfec_filings", [], |r| r.get(0))?;
        assert_eq!(style, "hdr");
        Ok(())
    }

    /// One filing per legacy format family (fixtures shared with
    /// fec-parser): every row is either exported or skipped with a warning,
    /// and libfec_filings records the header style and paper batch fields.
    #[test]
    fn test_legacy_families() -> anyhow::Result<()> {
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../fec-parser/tests/fixtures/legacy");
        let mut db = rusqlite::Connection::open_in_memory()?;
        let mut tx = db.transaction()?;
        init(&mut tx)?;
        for name in [
            "1.02_497.fec",
            "2.02_10665.fec",
            "3.00_13801.fec",
            "5.00_102196.fec",
            "5.3_300707.fec",
            "6.1_342096.fec",
            "7.0_730663.fec",
            "P1.0_236480.fec",
            "P2.4_391955.fec",
            "P2.6_716051.fec",
            "P3.2_1081726.fec",
            "P3.4_1215766.fec",
        ] {
            let bytes = std::fs::read(fixtures.join(name))?;
            let id = name.trim_end_matches(".fec").rsplit('_').next().unwrap_or(name);
            let filing = Filing::from_reader(bytes.as_slice(), id.to_owned(), bytes.len())?;
            insert_filing_metadata(&mut tx, &filing)?;
            export_itemizations(&mut tx, filing, None, None)?;
        }
        tx.commit()?;

        insta::assert_snapshot!(query(
            &db,
            "select filing_id, fec_version, software_name, cover_record_form, \
             cover_record_form_amendment_indicator, coverage_from_date, header_style, \
             batch_number, received_date from libfec_filings order by rowid"
        ));
        let tables: Vec<String> = db
            .prepare("select name from sqlite_master where type = 'table' order by name")?
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        let mut counts = String::new();
        for table in tables {
            let rows: Vec<(String, i64)> = db
                .prepare(&format!(
                    "select filing_id, count(*) from [{table}] group by 1 order by 1"
                ))?
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<_, _>>()?;
            for (filing_id, n) in rows {
                counts.push_str(&format!("{table} {filing_id} {n}\n"));
            }
        }
        insta::assert_snapshot!("legacy families row counts", counts);
        Ok(())
    }

    #[test]
    fn test_sqlite_docs_in_schema() {
        use super::super::{RecordTable, ALL_TABLES, LATEST_FEC_VERSION};

        let db = rusqlite::Connection::open_in_memory().unwrap();

        for &(row_type, suffix) in ALL_TABLES {
            match RecordTable::new(row_type, suffix, LATEST_FEC_VERSION) {
                Ok(rt) => {
                    db.execute(&rt.create_sql(), []).unwrap();
                }
                Err(_) => continue,
            }
        }

        let mut out = String::new();
        for &(_, suffix) in ALL_TABLES {
            let table_name = format!("libfec_{suffix}");
            let sql: Option<String> = db
                .query_row(
                    "select sql from sqlite_master where name = ?1",
                    [&table_name],
                    |row| row.get(0),
                )
                .ok();
            if let Some(sql) = sql {
                out.push_str(&sql);
                out.push_str("\n\n");
            }
        }

        insta::assert_snapshot!(out);
    }
}
