# Analyzing ActBlue & WinRed filings with libfec

[ActBlue](https://www.actblue.com/) ([`C00401224`](https://www.fec.gov/data/committee/C00401224/)) and [WinRed](https://winred.com/) ([`C00694323`](https://www.fec.gov/data/committee/C00694323/)) are the two largest federal campaign fundraising platforms for Democrats and Republicans, respectively. 

They act as [conduits](https://moneyinpolitics.wtf/conduit/) between individual donors and campaigns/committees. People donate *through* these online platforms (typically with credit cards) to whichever campaign or committee they care about. These [earmarked contributions](https://moneyinpolitics.wtf/earmarked-contribution/) are noted as such within FEC filings, and are reported in both ActBlue/WinRed filings (as disbursements) and committee filings (as receipts).

Because they are conduits, ActBlue and WinRed are required to report *all* earmarked contributions they receive. This is unique! Typically, campaign and PACs are only required to report individuals who donated $200 or more for a specific election, meaning small-dollar donors don't appear in committee filings. However, those small-dollar donors will appear in ActBlue/WinRed filings. 

## ActBlue and WinRed filings are *big*

A vast majority of PACs submit very small FEC filings, < `10KB`. Most well-funded House or Senate campaigns may include hundreds or thousands of contributors, but they filings will still be between `500KB` or `5MB` large. Presidential campaigns or political parties may have filings up to the 10's or 100's of MB, like Kamala Harris's `571 MB` [Post-General 2024 report](https://docquery.fec.gov/cgi-bin/forms/C00703975/1890563/).

But ActBlue and WinRed filings are on another level, consistently between `2GB` and `8GB`. Their [Mid-Year 2025 report](https://docquery.fec.gov/dcdev/posted/1909062.fec) alone clocks in at `10.3 GB`.

