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
        export_itemizations(&mut tx, f1s_84, None)?;
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
        export_itemizations(&mut tx, f1s_85, None)?;
        tx.commit()?;
        insta::assert_snapshot!(
            "F1S 8.5 data",
            query(&db, "select * from libfec_F1S where filing_id = '1923816'"),
        );

        Ok(())
    }
}
