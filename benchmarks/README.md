So far while benchmarking against `fastfec`, I get weird results. The fastfec README claims that parsing the `FEC-1464847` filing (8.4GB) takes `1m42s`. But on my machine, using both the Homebrew and [directly from releases](https://github.com/washingtonpost/FastFEC/releases), it took my computer `7m27s`. 

On the other hand, `libfec fastfec-compat` took `26s`.

I'm guessing that since `fastfec` uses an older verison of Zig, and I'm on a newer Mac (M4 vs M1), maybe that older build doesn't take advantage of the new hardware I'm on. It's hard to build `fastfec` from scratch now, since it's an older zig compiler.