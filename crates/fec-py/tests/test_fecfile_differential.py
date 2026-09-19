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

from libfec_parser import FecParseError
from libfec_parser import fecfile as ours

real = pytest.importorskip("fecfile")


# --- The allowlist --------------------------------------------------------
#
# Two differences — (a) needs a helper per API — and nothing is excluded from a
# comparison without appearing here.  (a) is a deferred parser item the bindings
# are explicitly not allowed to work around; (b) is a bug in the real package,
# which we decline to reproduce.


def without_f99_text(parsed: dict[str, Any]) -> dict[str, Any]:
    """(a) Drop real's ``F99_text`` key.

    A form 99's narrative sits between ``[BEGINTEXT]`` and ``[ENDTEXT]`` markers
    that `fec-parser` does not surface (deferred item N14), so ours has no
    ``F99_text`` to offer.  See also `without_f99_text_items` for the streaming
    side of the same gap.
    """
    parsed = dict(parsed)
    parsed.pop("F99_text", None)
    return parsed


def without_f99_text_items(items: list[Any]) -> list[Any]:
    """(a) again, as `iter_file` sees it: drop real's ``F99_text`` items."""
    return [item for item in items if item.data_type != "F99_text"]


def without_line_terminator(data: Any) -> Any:
    """(b) Strip the line terminator real leaves in a row's last field.

    Two shapes of the same bug in the real package, both of which reach the last
    field of every row it parses:

    * ``\\n`` from ``iter_file``/``iter_http``: `fecparser.iter_lines` reads a file
      object a line at a time and never strips the newline (``'20006\\n'``, or just
      ``'\\n'`` when the field is empty).
    * ``\\r`` from CRLF content handed to ``loads``/``iter_lines``: real splits it
      on ``'\\n'`` alone.  (``from_file``/``iter_file`` open the file in text mode,
      so universal newlines have already translated the ``\\r`` away there — a CRLF
      filing read from disk matches ours exactly, terminator included.)

    Neither is a difference of ours, and reproducing either would be silly.
    """
    if not isinstance(data, dict):
        return data
    return {
        key: value.removesuffix("\n").removesuffix("\r") if isinstance(value, str) else value
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
    """``(ours, real)`` for one fixture, with the allowlist applied to real's side.

    Nothing is dropped from ours: both allowlist entries are things real has and
    this package does not.
    """
    mine = ours.from_file(fec_fixture, options=options)
    theirs = without_f99_text(real.from_file(str(fec_fixture), options=options or {}))
    return mine, theirs


# --- The whole mapping space ----------------------------------------------
#
# The fixtures below exercise five filings' worth of forms.  These two tests
# exercise the *names*, which is where the compat layer and `fec-parser` are
# allowed to disagree: every concrete form type real's ``mappings.json``
# recognises, against every version this module claims to map.

#: A concrete row type for each of the 58 patterns in real's ``mappings.json``,
#: with the ``N``/``A``/``T`` (new/amended/termination) suffixes the forms that
#: take them actually appear with, and realistic line numbers on the schedules.
FORM_TYPES = [
    "HDR", "TEXT",
    "F1", "F1N", "F1A", "F1S",
    "F1M", "F1MN", "F1MA",
    "F2", "F2N", "F2A",
    "F24", "F24N", "F24A",
    "F3", "F3N", "F3A", "F3T",
    "F3L", "F3LN", "F3LA",
    "F3P", "F3PN", "F3PA", "F3PT", "F3P31", "F3PS", "F3PZ1", "F3PZ2",
    "F3S",
    "F3X", "F3XN", "F3XA", "F3XT",
    "F3Z", "F3ZT", "F3Z1", "F3Z2",
    "F4", "F4N", "F4A", "F4T",
    "F5", "F5N", "F5A", "F56", "F57",
    "F6", "F6N", "F6A", "F65",
    "F7", "F7N", "F7A", "F76",
    "F8", "F8N", "F8A", "F8II", "F8III",
    "F9", "F9N", "F9A", "F91", "F92", "F93", "F94",
    "F10", "F105",
    "F13", "F13N", "F13A", "F132", "F133",
    "F99",
    "H1", "H2", "H3", "H4", "H5", "H6",
    "SA11AI", "SA11AII", "SA11B", "SA11C", "SA12", "SA13", "SA14", "SA15",
    "SA16", "SA17", "SA3L", "SA3L-A",
    "SB17", "SB21B", "SB22", "SB23", "SB26", "SB27", "SB28A", "SB28B",
    "SB28C", "SB29", "SB30B",
    "SC/9", "SC/10", "SC1/9", "SC2/9",
    "SD9", "SD10",
    "SE", "SF", "SI", "SL",
]

#: The versions `fec-parser` reads a whole filing in.
VERSIONS_8X = ["8.5", "8.4", "8.3", "8.2", "8.1", "8.0"]

#: Older versions, which only reach this module through `parse_line`/`_mapping`
#: — `from_file`/`loads` reject the filing (see the README's scope paragraph).
#: Their mappings are compared all the same, and they match.
VERSIONS_LEGACY = ["7.0", "6.4", "6.1", "5.3", "5.0", "3.0"]


def real_names(form: str, version: str) -> list[str] | None:
    """Real's column names for one pair, or `None` where it has no mapping."""
    try:
        return list(real.fecparser.getMapping(real.fecparser.mappings, form, version))
    except real.FecParserMissingMappingError:
        return None


def our_names(form: str, version: str) -> list[str] | None:
    """Ours' column names — the translated ones that become dict keys."""
    try:
        return list(ours._mapping(form, version)[0])
    except ours.FecParserMissingMappingError:
        return None


@pytest.mark.parametrize("version", VERSIONS_8X + VERSIONS_LEGACY)
def test_column_names_match_real_across_mappings(version):
    """Same names, same order, same duplicates, for every form real maps.

    The fixtures only reach a dozen or so ``(form, version)`` pairs; this reaches
    every pattern in real's ``mappings.json``, which is what catches a name
    `fec-parser` spells differently (``col_a_total_receipts_TODO_DUP``,
    ``TODO_UNKNOWN_BLANK``) before a filing that uses that form does.  Real
    raising ``FecParserMissingMappingError`` counts as an answer: ours has to
    raise it for exactly the same pairs.
    """
    mismatches: list[tuple[str, str, Any]] = []
    mapped = 0
    for form in FORM_TYPES:
        want, got = real_names(form, version), our_names(form, version)
        if want is not None and got is not None and want != got:
            differing = [(i, a, b) for i, (a, b) in enumerate(zip(want, got)) if a != b]
            mismatches.append((form, f"{len(want)} vs {len(got)} columns", differing[:4]))
        elif (want is None) != (got is None):
            mismatches.append((form, "missing on one side", "real" if got else "ours"))
        elif want is not None:
            mapped += 1
    assert not mismatches, f"version {version}: {mismatches}"
    # Guard against the test quietly comparing nothing: most forms map in every
    # version this parametrises, and none of them maps fewer than half.
    assert mapped > len(FORM_TYPES) // 2, f"version {version}: only {mapped} forms mapped"


# The synthetic cover line below numbers every field, so its date columns warn
# on both sides; `test_type_warnings_match_real` is where the messages are
# compared, and here they are just noise.
@pytest.mark.filterwarnings("ignore::UserWarning")
def test_duplicate_cover_columns_take_the_last_value():
    """A column the mapping names twice keeps the *last* copy's value, like real.

    An F3X cover names ``col_a_total_receipts`` (and five more) twice, and an F2
    names ``candidate_state`` twice.  Real's ``out[k] = ...`` loop leaves the last
    occurrence's value at the first occurrence's key position; `fec-parser`
    disambiguates the second copy as ``*_TODO_DUP`` instead, so this module has
    to translate the name back before building the dict, or a filer whose two
    totals differ reads the wrong one.  Built by hand: every committed fixture
    happens to repeat the same number in both copies, which hides this entirely.
    """
    names = real_names("F3XN", "8.5")
    assert names is not None
    assert len(set(names)) < len(names), "F3XN 8.5 is supposed to name a column twice"
    # One distinct value per position, so the two copies of a column can't agree.
    fields = ["F3XN"] + [str(i) for i in range(1, len(names))]
    line = "\x1c".join(fields)
    document = "\n".join(["\x1c".join(["HDR", "FEC", "8.5", "unit-test", "1"]), line])

    assert_record_equal(
        ours.parse_line(line, "8.5") or {},
        real.parse_line(line, "8.5"),
        "F3XN duplicate columns, typed",
    )
    for as_strings in (False, True):
        options = {"as_strings": as_strings}
        assert_record_equal(
            ours.loads(document, options=options)["filing"],
            real.loads(document, options=options)["filing"],
            f"F3XN duplicate columns, as_strings={as_strings}",
        )
    # And the value really is the last copy's, not the first's.
    mine = ours.parse_line(line, "8.5") or {}
    last = len(names) - 1 - names[::-1].index("col_a_total_receipts")
    assert mine["col_a_total_receipts"] == float(fields[last])


def test_blank_column_name_matches_real():
    """The F3L mapping's nameless column is keyed ``''``, as real keys it."""
    line = "\x1c".join(["F3LN", "C00000000"])
    mine, theirs = ours.parse_line(line, "8.5"), real.parse_line(line, "8.5")
    assert mine is not None
    assert "" in theirs, "F3LN 8.5 is supposed to have an empty-string column name"
    assert_record_equal(mine, theirs, "F3LN blank column name")


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
    assert_parsed_equal(lower, without_f99_text(upper), fec_fixture.name)


def test_loads_matches_from_file(fec_fixture):
    """``str``, ``bytes`` and a list of lines all land on the same dict."""
    raw = fec_fixture.read_bytes()
    text = raw.decode("utf-8")
    from_file = ours.from_file(fec_fixture)
    for label, source in [
        ("str", text),
        ("bytes", raw),
        ("list[str]", text.split("\n")),
        ("list[bytes]", raw.split(b"\n")),
    ]:
        assert_parsed_equal(ours.loads(source), from_file, f"{fec_fixture.name} loads({label})")


def test_iter_file_matches_real(fec_fixture):
    mine = list(ours.iter_file(fec_fixture))
    theirs = without_f99_text_items(list(real.iter_file(str(fec_fixture))))

    assert [item.data_type for item in mine] == [item.data_type for item in theirs]
    for i, (have, want) in enumerate(zip(mine, theirs)):
        where = f"{fec_fixture.name} item[{i}] ({want.data_type})"
        assert_record_equal(have.data, without_line_terminator(want.data), where)


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
        mine = ours.loads(lines, options=options)
        theirs = without_f99_text(real.loads(document, options=options or {}))
        assert_parsed_equal(mine, theirs, f"TEXT filter={prefixes}")

    # And the row really is there to be filtered in the first place.
    assert len(ours.loads(lines)["text"]) == 1
    assert ours.loads(lines, options={"filter_itemizations": ["SA"]})["text"] == []


# --- Lines that are not records -------------------------------------------


#: Lines real `fecfile` refuses to turn into a record, one per reason.
NON_RECORD_LINES = [
    pytest.param("SA11AI", id="one-field"),
    pytest.param("   ", id="whitespace-only"),
    pytest.param("", id="empty"),
    pytest.param("\t", id="tab"),
]


@pytest.mark.parametrize("line", NON_RECORD_LINES)
def test_non_record_lines_are_skipped(sample_fec_content, line):
    """A line with no delimiter, or only whitespace, is skipped -- not an item.

    Real's ``parse_line`` returns ``None`` for a line with fewer than two fields
    and ``iter_lines`` drops it (`fecparser.py:107-108`).  Ours used to hand a
    delimiter-free ``'SA11AI'`` through as a one-field Schedule A itemization,
    and to raise `FecParserMissingMappingError` for a whitespace-only line,
    because the native reader raises `MissingMappingError` for a blank row type.
    Both now skip, and the native reader is still usable afterwards.
    """
    header, cover = sample_fec_content.split("\n")[:2]
    # A real itemization after the bad line, so "skipped" is distinguishable
    # from "stopped reading here".
    tail = "\x1c".join(["SA11AI", "C00900860", "SA11.4000"])
    lines = [header, cover, line, tail]

    for prefixes in (None, ["SA"]):
        options = None if prefixes is None else {"filter_itemizations": prefixes}
        mine = ours.loads(lines, options=options)
        theirs = without_f99_text(real.loads(lines, options=options or {}))
        assert_parsed_equal(mine, theirs, f"non-record {line!r} filter={prefixes}")
        assert len(mine["itemizations"]["Schedule A"]) == 1

    # The streaming path skips it too, and stays in step with the eager one.
    assert [item.data_type for item in ours.iter_lines(lines)] == [
        "header",
        "summary",
        "itemization",
    ]


def test_missing_mapping_still_raises(sample_fec_content):
    """Skipping blank row types does not swallow a genuinely unmapped form."""
    header, cover = sample_fec_content.split("\n")[:2]
    lines = [header, cover, "\x1c".join(["ZZ99", "C00900860"])]
    with pytest.raises(ours.FecParserMissingMappingError) as mine:
        ours.loads(lines)
    with pytest.raises(real.FecParserMissingMappingError) as theirs:
        real.loads(lines)
    assert str(mine.value) == str(theirs.value)


# --- CRLF ------------------------------------------------------------------


def test_crlf_file_matches_real(fec_fixture, tmp_path):
    """A CRLF filing read from disk matches real exactly, terminators included.

    Real opens the file in text mode, so universal newlines have already turned
    ``\\r\\n`` into ``\\n`` before `fecparser` ever sees it; nothing is left in the
    last field, and neither side is allowed any slack here.  Built from a
    committed fixture rather than committing a second copy of one.
    """
    crlf = tmp_path / fec_fixture.name
    crlf.write_bytes(fec_fixture.read_bytes().replace(b"\n", b"\r\n"))

    assert_parsed_equal(
        ours.from_file(crlf),
        without_f99_text(real.from_file(str(crlf))),
        f"{fec_fixture.name} CRLF from_file",
    )
    # …and the same filing with and without the \r is the same filing.
    assert_parsed_equal(ours.from_file(crlf), ours.from_file(fec_fixture), fec_fixture.name)

    mine = list(ours.iter_file(crlf))
    theirs = without_f99_text_items(list(real.iter_file(str(crlf))))
    assert [i.data_type for i in mine] == [i.data_type for i in theirs]
    for i, (have, want) in enumerate(zip(mine, theirs)):
        # Allowlist (b): real's own `iter_file` leaves the \n in the last field.
        assert_record_equal(
            have.data, without_line_terminator(want.data), f"{fec_fixture.name} CRLF item[{i}]"
        )


def test_crlf_in_memory_matches_real(sample_fec_bytes):
    """CRLF content handed to ``loads`` matches real up to allowlist (b).

    Here the ``\\r`` really does survive into real's fields -- it splits on
    ``'\\n'`` alone -- so it is stripped from real's side before comparing.  Every
    other value, and the key order, has to match on the nose.
    """
    crlf = sample_fec_bytes.replace(b"\n", b"\r\n")
    theirs = real.loads(crlf.decode("utf-8"))
    theirs = {
        "itemizations": {
            group: [without_line_terminator(row) for row in rows]
            for group, rows in theirs["itemizations"].items()
        },
        "text": [without_line_terminator(row) for row in theirs["text"]],
        "header": without_line_terminator(theirs["header"]),
        "filing": without_line_terminator(theirs["filing"]),
    }
    for label, source in [("bytes", crlf), ("str", crlf.decode("utf-8"))]:
        assert_parsed_equal(ours.loads(source), theirs, f"CRLF loads({label})")


# --- Scope: which FEC format versions the whole-filing APIs read -----------


#: A version real `fecfile` parses a whole filing in and `fec-parser` does not,
#: with the field separator that generation used (`fecparser.py:36` — versions
#: 1, 2, 3 and 5 are comma-separated, 6.x on use ASCII 28).
@pytest.mark.parametrize(
    ("version", "separator"),
    [("3.0", ","), ("5.3", ","), ("6.4", "\x1c"), ("7.0", "\x1c"), ("P3.4", "\x1c")],
)
def test_pre_8x_filings_are_out_of_scope(version, separator):
    """The whole-filing APIs read FEC 8.0-8.5 only; real reads every generation.

    `fec-parser` supports 8.0 through 8.5, so `from_file`/`loads`/`iter_file`
    raise `FecParseError` on anything older (or on a paper filing's ``P3.4``),
    where real parses it happily.  ``parse_line``/``parse_header`` are not
    limited this way -- they go straight to the mapping, which covers every
    version real's does, as `test_column_names_match_real_across_mappings`
    checks.  Pinned here so the README's scope paragraph and the code cannot
    drift apart silently.
    """
    document = separator.join(["HDR", "FEC", version, "unit-test", "1"]) + "\n"
    document += separator.join(["F3N", "C00000000", "A COMMITTEE"])

    assert real.loads(document)["filing"]["form_type"] == "F3N"
    for api in (ours.loads, lambda d: list(ours.iter_lines(d.split("\n")))):
        with pytest.raises(FecParseError):
            api(document)

    # But the single-line APIs do read it, and agree with real line for line.
    for line in document.split("\n"):
        mine, theirs = ours.parse_line(line, version), real.parse_line(line, version)
        assert mine is not None
        assert_record_equal(mine, theirs, f"parse_line({version})")


# --- Time zones ------------------------------------------------------------


#: Dates `zoneinfo` (ours) and `pytz` (real's) localize identically: every day
#: from 1901-12-14 (pytz's transition table starts at the 32-bit ``time_t``
#: minimum, 1901-12-13) through 2038-03-14 (it stops before the 2038 DST
#: change).  Both boundaries are measured, not assumed -- see
#: `test_timezone_out_of_range_divergences`.
IN_RANGE_DATES = [
    "19011214",  # the first day pytz has a real transition for
    "19500101",
    "19700101",
    "19990404",  # DST starts
    "20070311",  # DST starts, first year of the current US rule
    "20071104",  # DST ends
    "20230701",
    "20371231",
    "20380314",  # the last day the two agree
]


def contribution_dates(value: str) -> tuple[datetime, datetime]:
    """``(ours, real)`` for one ``YYYYMMDD`` in a Schedule A's date column."""
    columns = list(real.parse_line("SA11AI\x1cC00000000", "8.5"))
    fields = [""] * len(columns)
    fields[0] = "SA11AI"
    fields[columns.index("contribution_date")] = value
    line = "\x1c".join(fields)

    mine = (ours.parse_line(line, "8.5") or {})["contribution_date"]
    theirs = real.parse_line(line, "8.5")["contribution_date"]
    assert isinstance(mine, datetime) and isinstance(theirs, datetime)
    return mine, theirs


@pytest.mark.parametrize("value", IN_RANGE_DATES)
def test_timezone_matches_real_in_range(value):
    """Same instant, same offset, same repr, for every date a filing can hold."""
    mine, theirs = contribution_dates(value)
    assert mine == theirs
    assert mine.utcoffset() == theirs.utcoffset()
    assert str(mine) == str(theirs)
    assert hash(mine) == hash(theirs)


#: The two known windows where `pytz` and `zoneinfo` disagree, as measured on
#: `pytz` 2026.3: outside them the whole 1901-2038 span agrees day for day.
OUT_OF_RANGE_DATES = [
    # pytz has no transition before 1901-12-13, so it reads Local Mean Time
    # where zoneinfo has had standard time since 1883-11-18.
    ("18830701", "-04:56", "-04:56:02"),
    ("19011213", "-04:56", "-05:00"),
    # …and none after 2037, so every summer date from 2038 on stays on EST.
    ("20380315", "-05:00", "-04:00"),
    ("20380701", "-05:00", "-04:00"),
    ("20490701", "-05:00", "-04:00"),
]


@pytest.mark.parametrize(("value", "real_offset", "our_offset"), OUT_OF_RANGE_DATES)
def test_timezone_out_of_range_divergences(value, real_offset, our_offset):
    """The two documented windows where ours and real land on different offsets.

    Not an allowlist entry -- no test excludes these from a comparison -- but a
    real difference, pinned so that a `pytz` release, a tzdata update or a move
    off `zoneinfo` shows up here rather than in someone's filing.  No FEC filing
    carries a date in either window; the README says so in "Where it differs".
    """
    mine, theirs = contribution_dates(value)
    assert str(theirs).endswith(real_offset), str(theirs)
    assert str(mine).endswith(our_offset), str(mine)
    assert mine != theirs


# --- Malformed input -------------------------------------------------------
#
# Four places where a *malformed* filing is treated differently.  None is an
# allowlist entry -- nothing above excludes them from a comparison -- but each
# is a real difference, pinned here and listed in the README so it is found on
# purpose rather than in someone's filing.  Every one of them needs input no
# valid filing contains.


def test_summary_must_be_a_cover_record(sample_fec_content):
    """The first record has to be a cover `fec-parser` knows, with a filer name.

    Real treats whatever it parses first as the summary, whatever it is; the
    native reader validates it as a cover before the compat layer sees anything.
    """
    header = sample_fec_content.split("\n")[0]
    for first, message in [
        ("SA11AI\x1cC00900860", "SA11AI"),  # an itemization as the first record
        ("F3N\x1cC00900860", "F3N"),  # a cover with no filer name
    ]:
        assert real.loads([header, first])["filing"]["form_type"] == message
        with pytest.raises(FecParseError):
            ours.loads([header, first])


def test_form_type_with_whitespace_is_unmapped(sample_fec_content):
    """Real strips the form for the mapping lookup; `fec-parser` does not.

    Real keeps the *unstripped* spelling as the ``form_type`` value, so its
    itemization group is ``' SA11AI '``; ours raises instead.
    """
    header, cover = sample_fec_content.split("\n")[:2]
    lines = [header, cover, " SA11AI \x1cC00900860"]
    assert list(real.loads(lines)["itemizations"]) == [" SA11AI "]
    with pytest.raises(ours.FecParserMissingMappingError):
        ours.loads(lines)


def test_filter_prefixes_match_the_row_type_not_the_line(sample_fec_content):
    """``filter_itemizations`` is matched against the row type, not the raw line.

    Real tests ``line.startswith(prefix)`` or ``line.startswith('"' + prefix)``,
    so a prefix that runs past the first field, or that carries the quote of a
    quoted row type, can match there and never here.  Prefixes that stay inside
    the row type -- every documented use -- behave identically, which
    `test_filter_itemizations_matches_real` covers.
    """
    header, cover = sample_fec_content.split("\n")[:2]
    for row, prefix in [
        ("SA11AI\x1cC00900860", "SA11AI\x1cC"),  # runs past the first field
        ('"SA11AI"\x1cC00900860', '"SA'),  # matches real's quoted-prefix branch
    ]:
        lines = [header, cover, row]
        options = {"filter_itemizations": [prefix]}
        theirs = real.loads(lines, options=options)["itemizations"]
        mine = ours.loads(lines, options=options)["itemizations"]
        assert sum(map(len, theirs.values())) == 1, prefix
        assert mine == {}, prefix


def test_blank_lines_shift_warning_line_numbers(sample_fec_content):
    """A blank line is not counted, so a later warning names an earlier line.

    Real counts every line it is handed; `fec-parser` numbers the *records* it
    reads and skips blank lines entirely, so ``Row.line`` -- and with it the
    ``(line N)`` a `FecParserTypeWarning` ends on -- falls behind by one per
    blank line above it.  The values, keys and warning text are identical; only
    the number differs.  `test_type_warnings_match_real` covers the no-blank-line
    case, which is every committed fixture and the benchmark filing.
    """
    lines = sample_fec_content.split("\n")
    columns = list(ours.parse_line(lines[2], "8.5") or {})
    fields = lines[2].split("\x1c")
    fields[columns.index("contribution_amount")] = "12,34.5x"
    document = "\n".join(lines[:2] + ["", "\x1c".join(fields)])

    def message(parse) -> str:
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            parse(document)
        return str(caught[0].message)

    assert message(real.loads).endswith("(line 5)")
    assert message(ours.loads).endswith("(line 4)")


def test_iterable_elements_are_not_record_boundaries(sample_fec_content):
    """An element of a ``loads``/``iter_lines`` iterable is text, not one record.

    Real parses each element as exactly one line; ours feeds the iterable to the
    parser as a byte stream, so an element containing ``\\n`` becomes two records.
    """
    header, cover = sample_fec_content.split("\n")[:2]
    row = "\x1c".join(["SA11AI", "C00900860"])
    lines = [header, cover, row + "\n" + row]
    assert sum(map(len, real.loads(lines)["itemizations"].values())) == 1
    assert sum(map(len, ours.loads(lines)["itemizations"].values())) == 2
