# libfec (Gleam)

Read FEC campaign finance filings (`.fec` files) from Gleam, with typed
records: every cover form and itemization family is a Gleam custom type you
can `case` on. Parsing is done by the Rust `fec-parser` crate, loaded as a
NIF. Erlang target only.

```gleam
case row.itemization {
  Some(itemization.ScheduleA(a)) if a.contribution_amount <. 0.0 -> refund(a)
  Some(itemization.ScheduleA(a)) -> contribution(a)
  Some(itemization.ScheduleB(b)) -> disbursement(b)
  _ -> other(row)
}
```

## Build

The NIF is built by cargo and copied to `priv/`. From `crates/fec-gleam`:

```sh
make            # release build -> gleam/priv/libfec_nif.so
make test       # build, then `gleam test`
```

Needs Rust, Gleam ≥ 1.14 and Erlang/OTP ≥ 26.

## Run the example

[`examples/summary`](examples/summary/src/summary.gleam) prints a filing's
typed cover, then folds once over its rows, matching on `ScheduleA`,
`ScheduleB`, `ScheduleE` and the rest. From `crates/fec-gleam`:

```sh
make example                         # crates/fec-py/tests/fixtures/1721696.fec
make example FEC=path/to/filing.fec
```

or, once the NIF is built, `gleam run -- path/to/filing.fec` in
`examples/summary`. It is its own Gleam project with `libfec` as a path
dependency, so the published package doesn't carry it.

## Tests

```sh
make test       # from crates/fec-gleam
```
