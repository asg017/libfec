use fec_parser_macros::{gen_column_names, gen_form_type_version_set, gen_form_types};
use regex::{RegexSet, RegexSetBuilder};

static FORM_TYPES: &[&str] = &gen_form_types!("");

lazy_static::lazy_static! {
  static ref FORM_TYPES_SET: RegexSet = RegexSetBuilder::new(FORM_TYPES)
    .case_insensitive(true)
    .build()
    .unwrap();
}
lazy_static::lazy_static! {
  static ref FORM_TYPE_VERSIONS_SET: Vec<RegexSet> = gen_form_type_version_set!("");
}

lazy_static::lazy_static! {
  static ref COLUMN_NAMES: Vec<Vec<Vec<String>>> = gen_column_names!();
}

pub fn field_idx(field: &str) -> Option<usize> {
    let matches = FORM_TYPES_SET.matches(field);
    matches.iter().next()
}

pub fn column_names_for_field<'a>(
    form_type: &str,
    fec_version: &str,
) -> std::result::Result<&'a Vec<String>, ()> {
    let idx = field_idx(form_type).unwrap();
    let idx2 = FORM_TYPE_VERSIONS_SET
        .get(idx)
        .unwrap()
        .matches(fec_version)
        .iter()
        .next()
        .unwrap();
    let columns = COLUMN_NAMES.get(idx).unwrap().get(idx2).unwrap();
    Ok(columns)
}

#[cfg(test)]
mod tests {
    use crate::*;
    use std::{
        fs::File,
        io::{BufRead, BufReader},
    };

    #[test]
    fn it_works() {
        assert_eq!(FORM_TYPES_SET.len(), FORM_TYPE_VERSIONS_SET.len());
        assert_eq!(FORM_TYPE_VERSIONS_SET.len(), COLUMN_NAMES.len());

        let fec_13360_19 = "SA11A1"; //"SA11A1,C00101766,IND,Kellner^Lawrence,10915 Pifer Way,,Houston,TX,77024,,,\"Continental Airlines, Inc.\",Exec. V.P. & CFO,5000.00,20000510,5000.00,,,,,,,,,,,,,,,,,A,SA11A1.7430";
        assert_eq!(field_idx(fec_13360_19), Some(44));
        assert_eq!(FORM_TYPES[44], "^sa");
        assert_eq!(
            FORM_TYPE_VERSIONS_SET
                .get(44)
                .unwrap()
                .matches("3")
                .iter()
                .next(),
            Some(11)
        );
        let x = &FORM_TYPE_VERSIONS_SET[44];

        assert_eq!(
            COLUMN_NAMES.get(44).unwrap().get(11).unwrap().join(","),
            "form_type,filer_committee_id_number,entity_type,contributor_name,contributor_street_1,contributor_street_2,contributor_city,contributor_state,contributor_zip_code,election_code,election_other_description,contributor_employer,contributor_occupation,contribution_aggregate,contribution_date,contribution_amount,contribution_purpose_code,contribution_purpose_descrip,donor_committee_fec_id,donor_candidate_fec_id,donor_candidate_name,donor_candidate_office,donor_candidate_state,donor_candidate_district,conduit_name,conduit_street1,conduit_street2,conduit_city,conduit_state,conduit_zip_code,memo_code,memo_text_description,amended_cd,transaction_id,back_reference_tran_id_number,back_reference_sched_name,reference_code"
        );
    }

    #[test]
    fn xxx() {
        let file = File::open("../tests/13360.fec").unwrap();
        let mut reader = BufReader::new(file);

        let mut line = String::with_capacity(1);
        let mut idx = 0;
        loop {
            let n = reader.read_line(&mut line);
            idx += 1;
            if idx >= 17 {
                break;
            }
        }

        let mut csv_reader = csv::ReaderBuilder::new()
            .flexible(true)
            .has_headers(false)
            .from_reader(reader);
        let x = csv_reader.records().next().unwrap().unwrap();
        assert_eq!(x.get(0), Some("F3XA"));
    }
}
