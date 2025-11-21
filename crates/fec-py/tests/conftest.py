"""
Pytest configuration and shared fixtures for libfec_parser tests
"""
import pytest
from pathlib import Path


def pytest_configure(config):
    """Configure pytest with custom markers"""
    config.addinivalue_line(
        "markers", "slow: marks tests as slow (deselect with '-m \"not slow\"')"
    )
    config.addinivalue_line(
        "markers", "network: marks tests that require network access"
    )


@pytest.fixture(scope="session")
def project_root():
    """Get the project root directory"""
    return Path(__file__).parent.parent.parent.parent


@pytest.fixture(scope="session")
def cache_dir(project_root):
    """Get the cache directory with FEC files"""
    return project_root / "cache"


@pytest.fixture(scope="session")
def benchmarks_dir(project_root):
    """Get the benchmarks directory with FEC files"""
    return project_root / "benchmarks"


@pytest.fixture(scope="session")
def available_fec_files(cache_dir, benchmarks_dir):
    """Get list of all available FEC test files"""
    files = []
    
    if cache_dir.exists():
        files.extend(list(cache_dir.glob("*.fec")))
    
    if benchmarks_dir.exists():
        files.extend(list(benchmarks_dir.glob("*.fec")))
    
    return files


@pytest.fixture
def first_fec_file(available_fec_files):
    """Get the first available FEC file for testing"""
    if not available_fec_files:
        pytest.skip("No FEC test files available")
    return available_fec_files[0]
