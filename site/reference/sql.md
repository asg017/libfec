
# `libfec` SQL Reference

> [!WARNING]
> This documentation is incomplete!

The [`libfec export`](./cli.md#export) command can export FEC filings into a SQLite database. All header, cover, and itemizations records from a FEC filing are inserted into various SQLite tables, allowing you to write SQL to extract out the exact information you need.


```bash
libfec export --committee=

```

The SQL tables that are created are based on 


## `libfec_filings`

<details>
  <summary> See <code>libfec_filings</code> SQL schema </summary>

  ```sql
  <!--@include: ./filings.sql-->
  ```

</details>


The `libfec_filings` table contains a single row for every FEC filing export. That rows contains the
(["header record"](https://docs.google.com/spreadsheets/d/1ZoUxmMw-X_I8DGMBYh-857hqgAzyYX2z/edit?gid=1892984062#gid=1892984062)) and "cover record" 
for a given filing. For example:

```bash
libfec export FEC-1848680  FEC-1870171 -o sanchez.db
```

The `libfec_filings` table inside of `sanchez.db` will have 2 rows in the table:

```sql
select 
  filing_id, 
  cover_record_form, 
  filer_id, 
  filer_name, 
  report_code
from libfec_filings;
```

```
┌───────────┬───────────────────┬───────────┬────────────────────┬─────────────┐
│ filing_id │ cover_record_form │ filer_id  │ filer_name         │ report_code │
├───────────┼───────────────────┼───────────┼────────────────────┼─────────────┤
│ 1870171   │ F3                │ C00384057 │ Stand With Sanchez │ YE          │
├───────────┼───────────────────┼───────────┼────────────────────┼─────────────┤
│ 1848680   │ F1                │ C00384057 │ Stand With Sanchez │             │
└───────────┴───────────────────┴───────────┴────────────────────┴─────────────┘
```

The first filing, 
[`FEC-1870171`](https://docquery.fec.gov/cgi-bin/forms/C00384057/1870171/)
, is the 
[FEC Form 3 "Year-End"](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf) 
financial report filed by 
[Stand with Sanchez](https://www.fec.gov/data/committee/C00384057)
, the principal campaign committee for Linda Sanchez (CA-38). The second filing, 
[`FEC-1848680`](https://docquery.fec.gov/cgi-bin/forms/C00384057/1848680/)
, is the 
[FEC Form 1 "Statement of Organization"](https://www.fec.gov/resources/cms-content/documents/fecfrm1sf.pdf) 
that the Sanchez campaign submitted in the 2024 election cycle.

The `libfec_filings` table is the core table for all the other exported itemizations. Every other table in the `sanchez.db` database (`libfec_F1`, `libfec_schedule_a`, etc.) all have foreign keys that point to the `libfec_filings.filing_id` primary key column. 

## Cover Tables

"Cover records" refer to the second line in a `.fec` file, which specifies which form the candidate or committee submitted. When exporting to a SQLite database, A every form type will have it's own table. 

In the `sanchez.db` example from above, there are two cover record tables: `libfec_F1` and `libfec_F3`.

T
```sql

```

