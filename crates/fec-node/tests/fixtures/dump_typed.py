"""Dump what the Python bindings' `libfec.Filing` says about a filing as JSON,
for the JS parity tests (crates/fec-node/tests/parity.test.mjs and
crates/fec-wasm/tests/parity.test.mjs). Run via make-typed-json.sh.

Output:

    {"filer": {"form_type", "filer_id", "filer_name", "report_code",
               "coverage_from_date", "coverage_through_date"},
     "cover": <typed>,
     "cover_record": <record>,
     "itemizations": [<typed> | <error>, ...],
     "itemization_records": [<record> | <error>, ...]}

- <typed> is a typed object as `{"type": <class name>, **to_dict()}` (the
  `type` tag the JS layer uses), or an `Unknown` as
  `{"type": "Unknown", "form_type", "values"}`.
- <record> is `{"line", "row_type", "values"}`, `values` being the record by
  column name (Python's value rules).
- <error> is `{"error": "MissingMappingError", "row_type", "line"}` for a
  row with no column mapping. Python's iterators raise it and then carry on
  with the next row, so a dump records it in place and keeps going; the JS
  generators end at the throw, so the parity tests compare up to the first
  error and expect that throw.

Dates are ISO strings.
"""
import json
import sys

import libfec


def typed(value):
    if isinstance(value, libfec.Unknown):
        return {"type": "Unknown", "form_type": value.form_type, "values": value.to_dict()}
    return {"type": type(value).__name__, **value.to_dict()}


def record(r):
    return {"line": r.line, "row_type": r.row_type, "values": dict(r.items())}


def drain(iterator, convert):
    out = []
    while True:
        try:
            item = next(iterator)
        except StopIteration:
            return out
        except libfec.MissingMappingError as e:
            out.append({"error": "MissingMappingError", "row_type": e.row_type, "line": e.line})
            continue
        out.append(convert(item))


with libfec.Filing(sys.argv[1]) as f:
    out = {
        "filer": {
            "form_type": f.form_type,
            "filer_id": f.filer_id,
            "filer_name": f.filer_name,
            "report_code": f.report_code,
            "coverage_from_date": f.coverage_from_date,
            "coverage_through_date": f.coverage_through_date,
        },
        "cover": typed(f.cover),
        "cover_record": record(f.cover_record),
        "itemizations": drain(f.itemizations, typed),
        "itemization_records": drain(f.itemization_records, record),
    }
json.dump(out, sys.stdout, default=str, separators=(",", ":"), allow_nan=False)
