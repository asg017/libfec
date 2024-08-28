.echo on
.timer on
.bail on

begin;
create index idx_libfec_schedule_a_filing_id on libfec_schedule_a(filing_id);
create index idx_libfec_schedule_a_form_type on libfec_schedule_a(form_type);
create index idx_libfec_schedule_a_entity_type on libfec_schedule_a(entity_type);
create index idx_libfec_schedule_a_contributor_city on libfec_schedule_a(contributor_city);
create index idx_libfec_schedule_a_contributor_state on libfec_schedule_a(contributor_state);
create index idx_libfec_schedule_a_contributor_zip_code on libfec_schedule_a(contributor_zip_code);
create index idx_libfec_schedule_a_contribution_date on libfec_schedule_a(contribution_date);
create index idx_libfec_schedule_a_contribution_amount on libfec_schedule_a(contribution_amount);

create virtual table fts_libfec_schedule_a_names using fts5(
  contributor_first_name,
  contributor_last_name,
  content=libfec_schedule_a
);

insert into fts_libfec_schedule_a_names(rowid, contributor_first_name, contributor_last_name)
  select rowid, contributor_first_name, contributor_last_name
  from libfec_schedule_a;

commit;

-- idx_libfec_schedule_a_filing_id -- 15.571s
-- idx_libfec_schedule_a_form_type -- 16.608s
-- idx_libfec_schedule_a_entity_type -- 14.972s
-- idx_libfec_schedule_a_contributor_city -- 22.609s
-- idx_libfec_schedule_a_contributor_state -- 17.643s
-- idx_libfec_schedule_a_contributor_zip_code -- 22.469s
-- idx_libfec_schedule_a_contribution_date -- 22.672s
-- idx_libfec_schedule_a_contribution_amount -- 20.950s
-- insert into fts_libfec_schedule_a_names (...) 39.382s
