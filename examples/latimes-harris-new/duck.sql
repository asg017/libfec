install sqlite;
load sqlite;

attach database 'actblue-all.db' as base (type sqlite);

copy (
  select

  rowid, filing_id, form_type, filer_committee_id_number, transaction_id,
  entity_type,
  --lower(printf('%s%s%s', contributor_first_name, contributor_last_name, substr(contributor_zip_code, 1, 5))) as contributor_id,
  contributor_first_name, contributor_last_name,
  contributor_street_1, contributor_street_2, contributor_city, contributor_state, contributor_zip_code, election_code, election_other_description,
  cast(contribution_date as text),
  contribution_amount, --contribution_aggregate,
  contributor_employer,
  contributor_occupation,
  memo_code, memo_text_description

  from base.libfec_schedule_a
  where entity_type = 'IND'
) to 'actblue-individual.parquet';
