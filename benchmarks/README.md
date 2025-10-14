# `libfec` Benchmarks

The `libfec` tool is very fast! Along with the [builtin cache](./guides/cache), `libfec` is likely the fastest tool for working with FEC filings.

For example, the largest single FEC filing I could find is [`FEC-1909062`](https://docquery.fec.gov/cgi-bin/forms/C00401224/1909062/), the `10GB` Mid-Year 2025 report for ActBlue. To export all reported contributions in that filing, you can run:

```bash
libfec export 1909062.fec \
  --target receipts \
  -o ab-2025-h1.csv
```

On my machine (2024 MacBook Pro M4 Pro) this takes 31 seconds, exporting a `5.8 GB` CSV with 25 million rows!

Other formats are supported to, like SQLite:

```bash
libfec export 1909062.fec \
  -o ab-2025-h1.db
```

This is slower, clocking in at 2 minutes 10 seconds, generating a `11GB` file. Exporting to SQLite will import all itemizations, including receipts and disbursements (while the above CSV export only export receipts). Also inserting into a SQLite table is always slower than writing a CSV.

## Compared with `fastfec`

The most similar tool to `libfec` is [`fastfec` from the Washington Post](https://github.com/washingtonpost/FastFEC). They both do very different things, so drawing a direct comparison is hard to do. But `libfec` has a [`libfec fastfec`](https://alexgarcia.xyz/libfec/guides/fastfec.html) subcommand that aims to do the same thing as `fastfec` (convert a single `.fec` file to a directory of CSVs by form type).

For example, a sample `fastfec` command would be:

```bash
fastfec 1805248.fec output/
```

And the `libfec fastfec` version would look like:

```bash
libfec fastfec 1805248.fec output/
```

Now, 


