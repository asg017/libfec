.timer on

.open tmp.db

attach database 'actblue-all.db' as source;

create table biden_harris_itemizations as
  select
    *,
    substr(contributor_zip_code, 1, 5) as contributor_zip_code5,
    lower(
      format(
        '%s%s%s',
        contributor_first_name,
        contributor_last_name,
        substr(contributor_zip_code, 1, 5)
      )
    ) as contributor_id
  from source.libfec_schedule_a
  where (
      memo_text_description like '%C00703975%'
      or memo_text_description like '%C00744946%'
      or memo_text_description like '%C00849281%'
      or memo_text_description like '%C00838912%'
  );
