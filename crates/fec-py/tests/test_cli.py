"""
The `libfec` command that the wheel installs: a console script into the extension
(``libfec._cli:main`` -> ``_native.run_cli``), not a separate binary.

Each test runs the CLI in a subprocess, the way the installed script would, since the
CLI may exit the process itself (clap's ``--help``/usage errors).
"""
import sqlite3
import subprocess
import sys
from importlib.metadata import entry_points

import libfec


def run_cli(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, "-c", "from libfec._cli import main; main()", *args],
        capture_output=True,
        text=True,
        timeout=60,
    )


def test_console_script_is_registered():
    """The wheel's `libfec` script points at libfec._cli:main"""
    (script,) = entry_points(group="console_scripts", name="libfec")
    assert script.value == "libfec._cli:main"


def test_version_matches_package():
    """`libfec --version` reports the same version as libfec.__version__"""
    result = run_cli("--version")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == f"libfec {libfec.__version__}"


def test_unknown_command_exits_2():
    """A usage error exits 2, like the native binary"""
    result = run_cli("not-a-command")
    assert result.returncode == 2
    assert "unrecognized command 'not-a-command'" in result.stderr


def test_export_to_sqlite(sample_fec_file, tmp_path):
    """`libfec export` writes the filing's rows to SQLite"""
    db = tmp_path / "out.db"
    result = run_cli("export", str(sample_fec_file), "-o", str(db))
    assert result.returncode == 0, result.stderr
    with sqlite3.connect(db) as conn:
        (filing_id,) = conn.execute("select filing_id from libfec_filings").fetchone()
    assert filing_id == "1921705"
