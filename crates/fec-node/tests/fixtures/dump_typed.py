"""Dump the Python bindings' typed records for a filing as JSON, for the
Node parity test (tests/parity.test.mjs). Run via make-typed-json.sh.

Output: {"cover": <dict|null>, "cover_row": {...}, "rows": [{"line",
"row_type", "values", "itemization"}]} for every mapped row, in file order.
`values` is the row by column name (Python's value rules); `itemization` is
the typed record or null, with `"type": <class name>`, the tag the JS layer
uses. Dates are ISO strings.
"""
import json
import sys

import libfec_parser as l


def typed(value):
    if value is None:
        return None
    return {"type": type(value).__name__, **value.to_dict()}


reader = l.open(sys.argv[1])
rows = []
while True:
    try:
        row = next(reader)
    except StopIteration:
        break
    except l.MissingMappingError:
        continue  # unmapped rows aren't typed; the reader moves on
    rows.append(
        {
            "line": row.line,
            "row_type": row.row_type,
            "values": dict(row.items()),
            "itemization": typed(row.itemization),
        }
    )
out = {
    "cover": typed(reader.cover_data),
    "cover_row": dict(reader.cover_row.items()),
    "rows": rows,
}
json.dump(out, sys.stdout, default=str, separators=(",", ":"), allow_nan=False)
