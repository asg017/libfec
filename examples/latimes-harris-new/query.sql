.timer on

.open tmp.db

create temp table contributor_stats as
select
  contributor_id,
  contributor_state,
  contributor_zip_code5,
  count(*) filter (where contribution_date < '2024-07-21') > 0 as biden_donor,
  count(*) filter (where contribution_date >= '2024-07-21') > 0 as harris_donor,
  sum(contribution_amount) as total_contribution_amount,
  sum(contribution_amount) filter (where contribution_date < '2024-07-21') > 0 as biden_contribution_amount,
  sum(contribution_amount) filter (where contribution_date >= '2024-07-21') > 0 as harris_contribution_amount

from biden_harris_itemizations
group by 1;


select
  count(distinct contributor_id),
  sum(biden_donor),
  sum(harris_donor),
  sum(biden_donor and harris_donor),
  sum(biden_donor and not harris_donor),
  sum(not biden_donor and harris_donor),
  sum(not biden_donor and not harris_donor)
from temp.contributor_stats;
