---
# https://vitepress.dev/reference/default-theme-home-page
layout: home

hero:
  name: "libfec"
  text: ""
  tagline: Wrangle federal campaign finance data from the command line
  actions:
    - theme: brand
      text: Installing
      link: /getting-started/installation
    - theme: alt
      text: Examples
      link: /examples

features:
  - title: Parse FEC filings!
    details: Convert .fec files into SQLite, Excel, CSVs, etc.
  - title: Works with the OpenFEC API
    details: Automatically download all filings for a given committee/candidate in one command
  - title: Really fast!
    details: Parses large ActBlue + WinRed filings in 30 seconds or less
---

