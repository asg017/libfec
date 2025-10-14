# FastFEC Compatability

[FastFEC](https://github.com/washingtonpost/FastFEC/) is popular a CLI tool for converting FEC filings to CSV files, created by the Washington Post in 2021. The `libfec` CLI contains a compatible CLI command to the `fastfec` CLI, aptly named `libfec fastfec`. 

The `libfec fastfec` command is meant for users who already use `fastfec` to try out `libfec` in their workflows, without completely re-writing their pipelines. 

For example, Say we are working with [`FEC-1781583`, Nikki Haley's March 2024 FEC filing](https://docquery.fec.gov/cgi-bin/forms/C00833392/1781583/). We can download the raw `.fec` file like so:

```bash
curl -o 1781583.fec 'https://docquery.fec.gov/dcdev/posted/1781583.fec'
```

We can convert that `23MB` `.fec` file into a directory of CSVs with `fastfec` like so:

```bash
$ fastfec 1781583.fec output/
About to parse filing ID 1781583
Trying filename: 1781583.fec
$ tree --du -h output
[ 19M]  output
└── [ 19M]  1781583
    ├── [5.4K]  F3PA.csv
    ├── [ 12M]  SA17A.csv
    ├── [1.3K]  SA17C.csv
    ├── [6.4M]  SA18.csv
    ├── [1.3K]  SA20A.csv
    ├── [177K]  SB23.csv
    ├── [3.2K]  SB28A.csv
    └── [ 132]  header.csv
```

That creates a new directory `output/1781583` with 8 CSVs, containing the itemization data from that filing.

Now with `libfec`, we can do a similar transformation with the `libfec fastfec` command:

```bash
$ libfec fastfec 1781583.fec output2
$ tree --du -h output2
[ 18M]  output2
└── [ 18M]  1781583
    ├── [5.4K]  F3PA.csv
    ├── [ 12M]  SA17A.csv
    ├── [1.3K]  SA17C.csv
    ├── [6.4M]  SA18.csv
    ├── [1.3K]  SA20A.csv
    ├── [176K]  SB23.csv
    ├── [3.2K]  SB28A.csv
    └── [ 133]  header.csv
```

## Additional `libfec fastfec` features

`fastfec` requires that the `.fec` file be pre-downloaded, but `libfec fastfec` can download FEC filings on the fly:


```bash
libfec fastfec FEC-1779004 output2/
```

This will download [`FEC-1779004`](https://docquery.fec.gov/cgi-bin/forms/C00833392/1779004/) file and write results to `output2/1779004` directly — no `curl` required!


## Benchmarks

Benchmarking `fastfec` is a bit complicated. Using the pre-compiled build from the [official `fastfec` Release](https://github.com/washingtonpost/FastFEC/releases/tag/0.4.1) seems to show that `fastfec` is ~11x slower than `libfec`:

```
Benchmark 1: ./fastfec -x 1805248.fec
  Time (mean ± σ):      4.754 s ±  0.024 s    [User: 4.686 s, System: 0.034 s]
  Range (min … max):    4.728 s …  4.803 s    10 runs
 
Benchmark 2: libfec fastfec 1805248.fec
  Time (mean ± σ):     411.9 ms ±  10.9 ms    [User: 347.0 ms, System: 52.7 ms]
  Range (min … max):   400.7 ms … 439.2 ms    10 runs
 
Summary
  libfec fastfec 1805248.fec ran
   11.54 ± 0.31 times faster than ./fastfec -x 1805248.fec
```

But this isn't 100% accurate. I believe the offical builds of `fastfec` use the debug compilation arguments, and could be faster if compiled with the `--release=fast` flag.

As an experiment, I built `fastfec` on my own machine, on top [of this PR](https://github.com/washingtonpost/FastFEC/pull/76), which gave faster results:


```
Benchmark 1: ../../FastFEC/zig-out/bin/fastfec -x 1805248.fec
  Time (mean ± σ):     714.4 ms ±  32.5 ms    [User: 678.2 ms, System: 23.3 ms]
  Range (min … max):   688.7 ms … 800.7 ms    10 runs
 
Benchmark 2: libfec fastfec 1805248.fec
  Time (mean ± σ):     394.4 ms ±   5.0 ms    [User: 339.8 ms, System: 50.1 ms]
  Range (min … max):   389.4 ms … 406.5 ms    10 runs
 
Summary
  libfec fastfec 1805248.fec ran
    1.81 ± 0.09 times faster than ../../FastFEC/zig-out/bin/fastfec -x 1805248.fec
```

Now this version `fastfec` is faster than the older version, but `libfec` is still `~1.8x` faster.

So I'd say in general, `libfec` is faster than `fastfec` in a head-to-head competition. But they're also very different tools, where `libfec` can resolve filings for committees and contests, while `fastfeec` just converts raw `.fec` files to a single CSV format.