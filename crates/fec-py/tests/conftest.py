"""
Pytest configuration and shared fixtures for libfec_parser tests.

All test data lives in ``tests/fixtures/`` and is committed to the repository
(see the ``!crates/fec-py/tests/fixtures/*.fec`` negation in the root
``.gitignore``).  A missing fixture is a hard failure, never a skip: tests that
silently skip are tests that never run in CI.
"""
import pytest
from pathlib import Path


FIXTURES = Path(__file__).parent / "fixtures"


def pytest_configure(config):
    """Register the opt-in markers."""
    config.addinivalue_line(
        "markers", "slow: parses the 91 MB benchmark filing; run with -m slow"
    )
    config.addinivalue_line(
        "markers", "network: hits fec.gov; run with -m network"
    )


def pytest_collection_modifyitems(config, items):
    """``network`` and ``slow`` tests are off unless explicitly selected."""
    selected = config.getoption("-m") or ""
    for item in items:
        for mark in ("network", "slow"):
            if mark in item.keywords and mark not in selected:
                item.add_marker(
                    pytest.mark.skip(reason=f"{mark} test; run with -m {mark}")
                )


def pytest_generate_tests(metafunc):
    """Parametrize any test asking for ``fec_fixture`` over every committed fixture."""
    if "fec_fixture" in metafunc.fixturenames:
        files = sorted(FIXTURES.glob("*.fec"))
        assert files, f"no fixtures found in {FIXTURES}"
        metafunc.parametrize("fec_fixture", files, ids=[p.name for p in files])


def _fixture(name: str) -> Path:
    p = FIXTURES / name
    # A missing fixture must FAIL, never skip.
    assert p.exists(), f"missing fixture {p}"
    return p


@pytest.fixture(scope="session")
def sample_fec_file() -> Path:
    """Primary fixture: 1921705.fec — v8.5, F3N, 20 itemizations, 4,022 bytes."""
    return _fixture("1921705.fec")


@pytest.fixture(scope="session")
def sample_fec_bytes(sample_fec_file) -> bytes:
    """The primary fixture's raw bytes."""
    return sample_fec_file.read_bytes()


@pytest.fixture(scope="session")
def sample_fec_content(sample_fec_bytes) -> str:
    """The primary fixture decoded as text.

    ``parse_header``/``parse_line`` accept ``str`` (or ``list[str]``) only, so
    this stays a string; use ``sample_fec_bytes`` for the bytes APIs.
    """
    return sample_fec_bytes.decode("utf-8")


@pytest.fixture(scope="session")
def pac_fec_file() -> Path:
    """1721696.fec — v8.4, F3XN, 1,387 itemizations, 263,105 bytes."""
    return _fixture("1721696.fec")


@pytest.fixture(scope="session")
def f99_fec_file() -> Path:
    """1913493.fec — v8.4, F99 with a [BEGINTEXT] block and zero itemizations."""
    return _fixture("1913493.fec")


@pytest.fixture(scope="session")
def all_fixture_files() -> list[Path]:
    """Every committed fixture, sorted by name."""
    return sorted(FIXTURES.glob("*.fec"))


@pytest.fixture(scope="session")
def benchmark_fec_file() -> Path:
    """The 91 MB benchmark filing — gitignored, opt-in via ``-m slow``."""
    p = Path(__file__).parents[3] / "benchmarks" / "1805248.fec"
    if not p.exists():
        pytest.skip("benchmarks/1805248.fec not present")
    return p
