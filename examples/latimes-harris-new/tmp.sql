.timer on

.open actblue.db
with harris_donors_90601 as (
select rowid, filing_id, form_type, filer_committee_id_number, transaction_id, back_reference_tran_id_number, back_reference_sched_name, entity_type, contributor_organization_name, contributor_last_name, contributor_first_name, contributor_middle_name, contributor_prefix, contributor_suffix, contributor_street_1, contributor_street_2, contributor_city, contributor_state, contributor_zip_code, election_code, election_other_description, contribution_date, contribution_amount, contribution_aggregate, contribution_purpose_descrip, contributor_employer, contributor_occupation,

  memo_text_description,
  reference_code
from libfec_schedule_a
where contribution_date >= '2024-07-21'
  and (
    memo_text_description like '%C00703975%'
    or memo_text_description like '%C00744946%'
    or memo_text_description like '%C00849281%'
    or memo_text_description like '%C00838912%'
  )
  and substr(contributor_zip_code, 1, 5) == '90601'
)
select
  count() as num_contributions,
  count(
    distinct printf('%s%s%s', contributor_first_name, contributor_last_name, substr(contributor_zip_code, 1, 5))
  ) as num_unique_contributors,
  sum(contribution_amount) as total_contributions
  from harris_donors_90601;


.open target.db
with harris_donors_90601 as (
  select rowid, filing_id, form_type, filer_committee_id_number, transaction_id,
  entity_type,
  lower(printf('%s%s%s', contributor_first_name, contributor_last_name, substr(contributor_zip_code, 1, 5))) as contributor_id,
  contributor_first_name, contributor_last_name,
  contributor_street_1, contributor_street_2, contributor_city, contributor_state, contributor_zip_code, election_code, election_other_description, contribution_date, contribution_amount, contribution_aggregate,
  contributor_employer,
  contributor_occupation,
  memo_code, memo_text_description
from libfec_schedule_a
where substr(contributor_zip_code, 1, 5) == '90601'
  --and lower(contributor_city) = 'whittier'
  and contribution_date >= '2024-07-21'
order by contributor_last_name
)
select
  count() as num_contributions,
  count(
    distinct printf('%s%s%s', contributor_first_name, contributor_last_name, substr(contributor_zip_code, 1, 5))
  ) as num_unique_contributors,
  sum(contribution_amount) as total_contributions
  from harris_donors_90601;
