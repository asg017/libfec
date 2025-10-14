---
# https://vitepress.dev/reference/default-theme-home-page
layout: home

hero:
  name: "libfec"
  text: ""
  tagline: A tool for wranging campaign finance data from the FEC
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
    details: Parses large ActBlue + WinRed Filings in 30 seconds or less
---

