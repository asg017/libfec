# `libfec`

A CLI for working with [FEC filings](https://www.fec.gov/help-candidates-and-committees/filing-reports/fecfile-software/), for building campaign finance data pipelines, news applications, and reporting tools.

- A single-binary CLI for MacOS, Linux, and Windows
- Outputs FEC filings to CSVs, JSON, Excel, or SQLite
- Only recent FEC filings version are support, ~2018-present (including the latest `8.5` version)
- Possible Python/Node.js/Ruby/WASM bindings in the future
- Really really fast!

```bash
# All financial activity from Elon Musk's America PAC from 2025-2026 to a SQLite database
libfec export C00879510 \
  --cycle 2026 \
  -o america-pac.db

# ActBlue's 2024 Post-General report as a directory of CSVs
libfec export \
  FEC-1857001 \
  --format csv \
  --output-directory output-actblue

# George Santos' 2022 House campaign disbursements as a CSV
libfec export C00721365 \
  --target disbursements \
  --cycle 2022 \
   -o santos22.csv
```

In the United States, all candidates running for federal office (House of Representatives, Senate, President, etc.), political action commitees (PACs), and political parties must periodically report financial information to the Federal Election Commission (FEC). These filings are publicly available, and gives the public insight into the people and groups funding federal election campaigns.

But these filings are extremely complex and hard to analyze. The [FEC website](https://www.fec.gov) offers different APIs and pre-packaged slices of all this data, but it can be hard to navigate or not up-to-date. So, `libfec` allows you to parse and export data directly from raw filings themselves, from the original `.fec` file format. 

There are already many open-source FEC parsers out there (see [Prior Art](#prior-art) for more info). So, `libfec` aims to be a fast, easy-to-use alternative that natively supports CSV, JSON, and SQLite exports!

## Installation

The recommended no-fuss installation method is with our installation script:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/asg017/libfec/releases/latest/download/fec-cli-installer.sh | sh
```

Alternatively, use the [`libfec` PyPi package](https://pypi.org/project/libfec/) with [`uv`](https://github.com/astral-sh/uv):

```sh
uvx libfec --help

# or use `uv tool` for a global install
uv tool install libfec
libfec --help
```

See [Installing `libfec`](https://alexgarcia.xyz/libfec/getting-started/installation) for all installation options. 

## Usage

To get all the itemizations from Nikki Haley's presidential campaign's [March 2024 FEC filing](https://docquery.fec.gov/cgi-bin/forms/C00833392/1781583/), you can run:

```bash
libfec export FEC-1781583 -o haley.db
```

And you'll have a SQLite database with all itemizations from that filing, including individual donors (above $200), committee transfers, operating expenses, loan information, and more. 

Under the hood, `libfec` will directly download and stream [the raw `1781583.fec` filing](https://docquery.fec.gov/dcdev/posted/1781583.fec), exporting the individual itemizations rows into SQLite tables. You can then use SQL to extract out the data you need, for visualizations, analysis, or data pipelines.

Or if you prefer an Excel workbook:

```bash
libfec export FEC-1781583 -o haley.xlsx
```

### All filings for a committee or campaign

Sometimes you don't care about a single filing — maybe you want *all* filings for a given candidate or committee. 

For example, for all filings made by Congresswoman Young Kim's (CA-39)  [principal campaign committee (`C00665638`)](https://www.fec.gov/data/committee/C00665638/) in the 2024 election cycle (Jan 2023 - Dec 2024), in [Form F3](https://www.fec.gov/resources/cms-content/documents/policy-guidance/fecfrm3.pdf), you could run:


```bash
libfec export C00665638 \
  --form-type=F3 \
  --cycle=2024 \
  -o kim.db
```

 This will use [the OpenFEC API](https://api.open.fec.gov/developers/) find all relevant filings and export them to `kim.db`. The result is a `34MB` SQLite database with 130k receipt itemizations (individual contributions, transfers, etc.) and nearly 5k disbursements itemizations. 

 Using this method, `libfec` will only fetch the most recent filings, and NOT outdated filings that were later amended.

### Analyzing parsed FEC filings

`libfec`'s SQLite support is the best way to analyze a candidate's or committee's financial activity. Using just SQL, you can extract out the exact data you want from a committee's filings, or analyze data directly in a standard way.

<!--
Virtually every programming language has their own client library to query SQLite databases — Python's [`sqlite3` module](https://docs.python.org/3/library/sqlite3.html), Node.js [`node:sqlite3` module](https://nodejs.org/api/sqlite.html), R's [`RSQLite` package](https://cran.r-project.org/web/packages/RSQLite/vignettes/RSQLite.html), Ruby's [`sqlite3` gem](https://github.com/sparklemotion/sqlite3-ruby). This means that even if your coworkers use a different programming language when analyzing data, you can still use SQL as a "lingua franca" for most 
-->

Using the `kim.db` example from above — which cities had the highest total donations from individual contributors to Kim’s campaign in 2024?


```sql
select 
  contributor_city, 
  contributor_state, 
  sum(contribution_amount)
from libfec_schedule_a
where form_type = 'SA11AI' -- itemized donations from individuals
  and entity_type = 'IND' -- exclude ActBlue memoized items
group by 1, 2
order by 3 desc
limit 10;
```

```
contributor_city    contributor_state      sum(contribution_amount)
------------------  -------------------  --------------------------
Los Angeles         CA                                     111615.0
Irvine              CA                                      93362.9
Newport Beach       CA                                      78302.3
New York            NY                                      75881.7
Washington          DC                                      66831.4
Anaheim             CA                                      62406.7
Fullerton           CA                                      59160.4
Santa Ana           CA                                      49312.7
Dallas              TX                                      45014.1
Houston             TX                                      42849.8
```

How much cash did Kim's campaign have throughout the cycle?

```sql
select
  coverage_from_date,
  coverage_through_date,
  -- format as USD currency
  format('$%,.2f', col_a_cash_on_hand_close_of_period) as cash_on_hand_end
 from libfec_F3
 order by 1;
 ```

 ```
coverage_from_date    coverage_through_date    cash_on_hand_end
--------------------  -----------------------  ------------------
2023-01-01            2023-03-31                 $902,615.71
2023-04-01            2023-06-30               $1,653,724.42
2023-07-01            2023-09-30               $2,223,485.38
2023-10-01            2023-12-31               $2,536,056.45
2024-01-01            2024-02-14               $2,509,006.41
2024-02-15            2024-03-31               $2,947,460.75
2024-04-01            2024-06-30               $3,610,109.15
2024-07-01            2024-09-30               $3,313,760.94
2024-10-01            2024-10-16               $2,822,770.30
2024-10-17            2024-11-25               $1,755,623.90
2024-11-26            2024-12-31               $1,737,498.43

```

How much did the campaign spend on WinRed fees?

```sql
select 
  format('$%,.2f', sum(expenditure_amount)) as total_spent
from libfec_schedule_b 
where payee_organization_name = 'WinRed Technical Services'
```

```
total_spent
-------------
$116,433.69
```

## Prior Art

There has been nearly 15 years of open source development on various FEC parsers, created by newsroom developers across the nation. Every new parser and tool has learned from it's predecessors, and `libfec` is no exception. 

Specifically, `libfec` adopted many features and configuration from the [FastFEC](https://github.com/washingtonpost/FastFEC) and [fecfile](https://github.com/esonderegger/fecfile) projects. 

Below are all the open source FEC file parsers and tools that I could readily find. Many haven't been updated in a while, but most still work!


| Repo                                      | Language      | Release date |
| ----------------------------------------- | ------------- | ------------ |
| https://github.com/cschnaars/FEC-Scraper  | Python+SQLite | ~2011        |
| https://github.com/dwillis/Fech           | Ruby          | ~2012?       |
| https://github.com/PublicI/fec-parse      | Node.js       | ~2015        |
| https://github.com/newsdev/fec2json       | Python        | ~2018        |
| https://github.com/esonderegger/fecfile   | Python        | ~2018        |
| https://github.com/washingtonpost/FastFEC | C/Python/WASM | ~2021        |
| https://github.com/NickCrews/feco3        | Rust          | ~2023        |