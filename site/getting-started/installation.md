#   Installing `libfec`

`libfec` is a command line interface (CLI) for interacting with federal campaign finance data from the FEC. While it's written in Rust, you personally don't need to know or even have Rust installed to use it. Pre-compiled binaries are made available for every `libfec` release for Windows, MacOS, and Linux users. 

Use one of the methods detailed below to install the `libfec` CLI on your machine.

## Recommended install script

The easiest way to install the `libfec` CLI is with the following installation script:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/asg017/libfec/releases/download/0.0.12/fec-cli-installer.sh | sh
```

Once ran, you can run the `libfec` command line by directly calling:

```sh
libfec --help
```

## With the `libfec` PyPi Package

If you're a Python user, the [`libfec` PyPi package](https://pypi.org/project/libfec/) includes pre-built binaries, so you can `pip install libfec` the CLI. 

If you use [uv](https://github.com/astral-sh/uv), you can use `uvx` for quick-and-dirty one-off scripts:

```bash
uvx libfec --help
```

Or use [`uv tool install`](https://docs.astral.sh/uv/reference/cli/#uv-tool-install) for an alternative global install:

```bash
uv tool install libfec
libfec --help
```

## Manually

Find a [recent release of `libfec`](https://github.com/asg017/libfec/releases) on the project's Releases page and manually download the correct package for your platform.

## Building yourself

Ensure you have Rust installed, then run:

```bash
cargo install --git https://github.com/asg017/libfec.git fec-cli
```



Alternatively you can manually clone the repository and run `cargo build --release`:

```bash
git clone git@github.com:asg017/libfec.git
cd libfec
cargo build --release
```

That will build the `libfec` CLI on your computer, and will be available at `target/release/libfec`. 