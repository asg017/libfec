"""
pandas interop for Filing.itemization_records (Mapping.register(Record), ticket 13).

No `importorskip`: a skipped test is a test that never runs (tests/conftest.py).
CI installs pandas alongside the wheel for the pytest step (test-python.yml).
"""
import pandas as pd  # type: ignore[import-untyped]  # no pandas-stubs dev dependency
from datetime import date

from libfec import Filing


def test_dataframe_from_rows_has_typed_columns(pac_fec_file):
    """pd.DataFrame(itemization_records) gets typed amount/date columns via Mapping.register(Record)"""
    df = pd.DataFrame(Filing(pac_fec_file).itemization_records)

    assert list(df.columns)[:3] == ["form_type", "filer_committee_id_number", "transaction_id"]
    assert df["contribution_amount"].dtype == "float64"  # all SA rows have a float or None
    assert isinstance(df["contribution_date"].dropna().iloc[0], date)  # object dtype of datetime.date; pandas does not auto-convert date -> datetime64
    assert pd.to_datetime(df["contribution_date"]).dt.year.min() == 2023
