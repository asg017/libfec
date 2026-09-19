"""``libfec_parser.fecfile`` against the real ``fecfile`` package, fixture by fixture.

This is the test that decides whether the compat layer is worth keeping: the
claim is that it is a *drop-in* for `fecfile` 0.9.1, so anything it does not
assert here is something we cannot promise.  Every value is compared by type as
well as by equality, and every dict by key order, so a ``'1.0'`` can never pass
for a ``1.0``.

The differences that survive are listed once, below, each with the reason.
"""
import warnings
from datetime import datetime
from typing import Any

import pytest

from libfec_parser import fecfile as ours

real = pytest.importorskip("fecfile")


# --- The allowlist --------------------------------------------------------
#
# Three differences — (b) needs a helper per API — and nothing is excluded from a
# comparison without appearing here.  (a) and (b) are deferred parser items the
# bindings are explicitly not allowed to work around; (c) is a bug in the real
# package, which we decline to reproduce.


def without_todo_dup(record: dict[str, Any]) -> dict[str, Any]:
    """(a) Drop ours' ``*_TODO_DUP`` cover columns.

    An F3X cover mapping names six columns twice; `fec-parser` keeps both copies
    and suffixes the second ``_TODO_DUP`` (deferred).  Real `fecfile` builds its
    dict by name, so the second copy simply overwrites the first — and that
    surviving value is the one ours has under the unsuffixed name too, which is
    why dropping the extras is enough to make the two comparable.
    """
    return {key: value for key, value in record.items() if not key.endswith("_TODO_DUP")}


def parsed_without_todo_dup(parsed: dict[str, Any]) -> dict[str, Any]:
    """(a) applied to the cover dict of a whole parsed filing."""
    parsed = dict(parsed)
    parsed["filing"] = without_todo_dup(parsed["filing"])
    return parsed


def without_f99_text(parsed: dict[str, Any]) -> dict[str, Any]:
    """(b) Drop real's ``F99_text`` key.

    A form 99's narrative sits between ``[BEGINTEXT]`` and ``[ENDTEXT]`` markers
    that `fec-parser` does not surface (deferred item N14), so ours has no
    ``F99_text`` to offer.  See also `without_f99_text_items` for the streaming
    side of the same gap.
    """
    parsed = dict(parsed)
    parsed.pop("F99_text", None)
    return parsed


def without_f99_text_items(items: list[Any]) -> list[Any]:
    """(b) again, as `iter_file` sees it: drop real's ``F99_text`` items."""
    return [item for item in items if item.data_type != "F99_text"]


def without_trailing_newline(data: Any) -> Any:
    """(c) Strip the line terminator real's ``iter_file`` leaves in a field.

    `fecparser.iter_lines` reads a file object a line at a time and never strips
    the ``\\n``, so the last field of every row it parses keeps one (``'20006\\n'``,
    or just ``'\\n'`` when the field is empty).  ``from_file``/``loads`` split the
    text on ``'\\n'`` first and are clean, so this is a bug in the real package,
    not a difference of ours, and reproducing it would be silly.
    """
    if not isinstance(data, dict):
        return data
    return {
        key: value.removesuffix("\n") if isinstance(value, str) else value
        for key, value in data.items()
    }


# --- Comparison helpers ---------------------------------------------------


def assert_record_equal(got: dict[str, Any], expected: dict[str, Any], where: str) -> None:
    """One parsed line: same columns, same order, same values, same types."""
    assert list(got) == list(expected), f"{where}: column names/order"
    for key, want in expected.items():
        have = got[key]
        assert type(have) is type(want), f"{where}[{key!r}]: {have!r} vs {want!r}"
        assert have == want, f"{where}[{key!r}]: {have!r} vs {want!r}"
        if isinstance(want, datetime):
            # Equal aware datetimes can still be written in different zones;
            # `zoneinfo` has to land on the same offset `pytz` did.
            assert have.utcoffset() == want.utcoffset(), f"{where}[{key!r}]: utcoffset"


def assert_parsed_equal(got: dict[str, Any], expected: dict[str, Any], where: str) -> None:
    """A whole parsed filing, including the order of the itemization groups."""
    assert list(got) == list(expected), f"{where}: top-level keys/order"
    assert_record_equal(got["header"], expected["header"], f"{where} header")
    assert_record_equal(got["filing"], expected["filing"], f"{where} filing")

    assert list(got["itemizations"]) == list(expected["itemizations"]), (
        f"{where}: itemization groups (file order)"
    )
    for group, rows in expected["itemizations"].items():
        assert len(got["itemizations"][group]) == len(rows), f"{where}: {group} row count"
        for i, row in enumerate(rows):
            assert_record_equal(got["itemizations"][group][i], row, f"{where} {group}[{i}]")

    assert len(got["text"]) == len(expected["text"]), f"{where}: text row count"
    for i, row in enumerate(expected["text"]):
        assert_record_equal(got["text"][i], row, f"{where} text[{i}]")


def parsed_pair(fec_fixture, options=None) -> tuple[dict[str, Any], dict[str, Any]]:
    """``(ours, real)`` for one fixture, with the allowlist applied to each side."""
    mine = parsed_without_todo_dup(ours.from_file(fec_fixture, options=options))
    theirs = without_f99_text(real.from_file(str(fec_fixture), options=options or {}))
    return mine, theirs


# --- The tests ------------------------------------------------------------


def test_from_file_matches_real(fec_fixture):
    mine, theirs = parsed_pair(fec_fixture)
    assert_parsed_equal(mine, theirs, fec_fixture.name)


def test_as_strings_matches_real(fec_fixture):
    mine, theirs = parsed_pair(fec_fixture, options={"as_strings": True})
    assert_parsed_equal(mine, theirs, fec_fixture.name)
    # Belt and braces: `as_strings` really does mean no coercion anywhere.
    assert all(isinstance(v, str) for v in mine["filing"].values())
    assert all(isinstance(v, str) for v in mine["header"].values())


@pytest.mark.parametrize("prefixes", [["SA"], ["SA", "SB"], []], ids=["SA", "SA+SB", "none"])
def test_filter_itemizations_matches_real(fec_fixture, prefixes):
    mine, theirs = parsed_pair(fec_fixture, options={"filter_itemizations": prefixes})
    assert_parsed_equal(mine, theirs, f"{fec_fixture.name} filter={prefixes}")


def test_lowercase_filter_matches_uppercase(fec_fixture):
    """``['sb']`` is ours alone — real is case-sensitive — so compare it to ``['SB']``."""
    lower = ours.from_file(fec_fixture, options={"filter_itemizations": ["sb"]})
    upper = real.from_file(str(fec_fixture), options={"filter_itemizations": ["SB"]})
    assert_parsed_equal(
        parsed_without_todo_dup(lower), without_f99_text(upper), fec_fixture.name
    )


def test_loads_matches_from_file(fec_fixture):
    """``str``, ``bytes`` and a list of lines all land on the same dict."""
    raw = fec_fixture.read_bytes()
    text = raw.decode("utf-8")
    from_file = parsed_without_todo_dup(ours.from_file(fec_fixture))
    for label, source in [
        ("str", text),
        ("bytes", raw),
        ("list[str]", text.split("\n")),
        ("list[bytes]", raw.split(b"\n")),
    ]:
        assert_parsed_equal(
            parsed_without_todo_dup(ours.loads(source)),
            from_file,
            f"{fec_fixture.name} loads({label})",
        )


def test_iter_file_matches_real(fec_fixture):
    mine = list(ours.iter_file(fec_fixture))
    theirs = without_f99_text_items(list(real.iter_file(str(fec_fixture))))

    assert [item.data_type for item in mine] == [item.data_type for item in theirs]
    for i, (have, want) in enumerate(zip(mine, theirs)):
        where = f"{fec_fixture.name} item[{i}] ({want.data_type})"
        assert_record_equal(
            without_todo_dup(have.data), without_trailing_newline(want.data), where
        )


def test_iter_file_matches_from_file(fec_fixture):
    """The streaming and eager paths agree — which `iter_lines` inherits."""
    streamed: dict[str, Any] = {"itemizations": {}, "text": [], "header": {}, "filing": {}}
    for item in ours.iter_file(fec_fixture):
        if item.data_type == "header":
            streamed["header"] = item.data
        elif item.data_type == "summary":
            streamed["filing"] = item.data
        elif item.data_type == "text":
            streamed["text"].append(item.data)
        else:
            form_type = item.data["form_type"]
            if form_type[0] == "S":
                form_type = "Schedule " + form_type[1]
            streamed["itemizations"].setdefault(form_type, []).append(item.data)
    assert_parsed_equal(streamed, ours.from_file(fec_fixture), fec_fixture.name)


def test_parse_header_matches_real(fec_fixture):
    lines = fec_fixture.read_text(encoding="utf-8").split("\n")
    mine = ours.parse_header(lines[0])
    theirs = real.parse_header(lines[0])
    assert mine[0] is not None
    assert_record_equal(mine[0], theirs[0], f"{fec_fixture.name} parse_header")
    assert mine[1:] == theirs[1:]
    # The list form has to agree with the single-line form, as it does for real.
    assert ours.parse_header(lines) == theirs


def test_parse_line_matches_real(sample_fec_file):
    """Every line of the primary fixture, one at a time."""
    lines = sample_fec_file.read_text(encoding="utf-8").split("\n")
    _, version, _ = ours.parse_header(lines[0])
    for i, line in enumerate(lines):
        mine = ours.parse_line(line, version)
        theirs = real.parse_line(line, version)
        if theirs is None:
            assert mine is None, f"line {i}"
            continue
        assert mine is not None, f"line {i}"
        assert_record_equal(mine, theirs, f"line {i}")


def test_print_example_matches_real(sample_fec_file, capsys):
    ours.print_example(ours.from_file(sample_fec_file))
    mine = capsys.readouterr().out
    real.print_example(real.from_file(str(sample_fec_file)))
    theirs = capsys.readouterr().out
    assert mine, "print_example printed nothing (and must be capturable)"
    assert mine == theirs


def test_type_warnings_match_real(sample_fec_content):
    """A value that does not parse warns with real's message, line number and all.

    Both packages number lines from 1 and then print ``line_num + 1`` (real's own
    off-by-one, `fecparser.py:71-77,219`), so the numbers only agree if ours
    feeds the warning a 1-based line too -- which is what ``Row.line`` gives.
    Built by hand rather than measured on a real filing because no committed
    fixture has a bad value, and neither does the 91 MB benchmark filing: all
    408,160 of its rows type cleanly, so the lockstep differential's warning
    assertion compares two empty lists and this is the test with teeth.
    """
    lines = sample_fec_content.split("\n")
    _, version, _ = ours.parse_header(lines[0])
    # lines[2] is the first SA11AI itemization; break its amount and its date.
    fields = lines[2].split("\x1c")
    columns = list(ours.parse_line(lines[2], version) or {})
    fields[columns.index("contribution_amount")] = "12,34.5x"
    fields[columns.index("contribution_date")] = "2023-99-99"
    lines[2] = "\x1c".join(fields)
    document = "\n".join(lines)

    def messages(parse, source) -> list[str]:
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            parse(source)
        return [str(w.message) for w in caught]

    mine = messages(ours.loads, document)
    theirs = messages(real.loads, document)
    assert len(mine) == 2, mine
    assert mine == theirs
    # And the line number really is in there, pointing at the line we broke.
    assert all(m.endswith("version: 8.5 (line 4)") for m in mine), mine


def test_missing_mapping_matches_real(sample_fec_file):
    """Same exception message, for a form/version pair neither package maps."""
    line = sample_fec_file.read_text(encoding="utf-8").split("\n")[2]
    with pytest.raises(ours.FecParserMissingMappingError) as mine:
        ours.parse_line(line, "99")
    with pytest.raises(real.FecParserMissingMappingError) as theirs:
        real.parse_line(line, "99")
    assert str(mine.value) == str(theirs.value)


def test_text_rows_follow_the_filter(sample_fec_content):
    """``TEXT`` rows are filtered by prefix like any other, as real does it.

    No committed fixture has one, so this builds a three-line filing by hand:
    header, cover, one ``TEXT`` row (which the mapping names ``rec_type``, so it
    lands in ``text`` rather than in ``itemizations``).
    """
    header, cover = sample_fec_content.split("\n")[:2]
    text_line = "\x1c".join(["TEXT", "C00900860", "T1", "SA1", "SA11AI", "a note"])
    lines = [header, cover, text_line]
    document = "\n".join(lines)

    for prefixes in (None, ["SA"], ["TEXT"]):
        options = None if prefixes is None else {"filter_itemizations": prefixes}
        mine = parsed_without_todo_dup(ours.loads(lines, options=options))
        theirs = without_f99_text(real.loads(document, options=options or {}))
        assert_parsed_equal(mine, theirs, f"TEXT filter={prefixes}")

    # And the row really is there to be filtered in the first place.
    assert len(ours.loads(lines)["text"]) == 1
    assert ours.loads(lines, options={"filter_itemizations": ["SA"]})["text"] == []
