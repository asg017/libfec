"""
pandas interop for Filing.rows (Mapping.register(Row), ticket 13).

No `importorskip`: a skipped test is a test that never runs (tests/conftest.py).
CI installs pandas alongside the wheel for the pytest step (test-python.yml).
"""
import pandas as pd  # type: ignore[import-untyped]  # no pandas-stubs dev dependency
from datetime import date

from libfec_parser import read


def test_dataframe_from_rows_has_typed_columns(pac_fec_file):
    """pd.DataFrame(filing.rows) gets typed amount/date columns via Mapping.register(Row)"""
    df = pd.DataFrame(read(pac_fec_file).rows)

    assert list(df.columns)[:3] == ["form_type", "filer_committee_id_number", "transaction_id"]
    assert df["contribution_amount"].dtype == "float64"  # all SA rows have a float or None
    assert isinstance(df["contribution_date"].dropna().iloc[0], date)  # object dtype of datetime.date; pandas does not auto-convert date -> datetime64
    assert pd.to_datetime(df["contribution_date"]).dt.year.min() == 2023
