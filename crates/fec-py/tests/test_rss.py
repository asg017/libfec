"""libfec.rss: the fec-rss crate's feed parser, plus urllib fetch()."""

import datetime
from pathlib import Path

import pytest

import libfec
from libfec import rss

# A trimmed copy of the live ?preDefinedFilingType=ALL feed (2026-10-10), shared
# with the fec-rss crate's own tests.
FEED = (
    Path(__file__).parents[2] / "fec-rss" / "tests" / "fixtures" / "all.xml"
).read_bytes()


def test_parse_bytes_and_str_agree() -> None:
    a = rss.parse_feed(FEED)
    b = rss.parse_feed(FEED.decode())
    assert [i.to_dict() for i in a] == [i.to_dict() for i in b]


def test_feed() -> None:
    feed = rss.parse_feed(FEED)
    assert feed.title == "FEC Electronic Filing RSS Feed - ALL"
    assert len(feed) == len(feed.items) == 30
    assert repr(feed) == "Feed(title=\"FEC Electronic Filing RSS Feed - ALL\", items=30)"


def test_item() -> None:
    item = rss.parse_feed(FEED).items[0]
    assert isinstance(item, rss.Item)
    assert item.committee_name == "ALLIED UNIVERSAL PAC"
    assert item.filing_id == "2019253"
    assert item.committee_id == "C00536425"
    assert item.form_type == "F3XN"
    assert item.report_type == "OCTOBER QUARTERLY"
    assert item.coverage_from == datetime.date(2026, 7, 1)
    assert item.coverage_through == datetime.date(2026, 9, 30)
    assert item.pub_date == datetime.datetime(
        2026, 10, 10, 17, 50, 22, tzinfo=datetime.timezone.utc
    )
    assert item.link == "http://docquery.fec.gov/dcdev/posted/2019253.fec"
    assert item.description.startswith("<p>The ALLIED UNIVERSAL PAC")
    assert set(item.to_dict()) == {
        "title", "link", "description", "pub_date", "guid", "committee_id",
        "filing_id", "form_type", "coverage_from", "coverage_through",
        "report_type", "committee_name",
    }


def test_entities_decoded() -> None:
    names = {i.committee_name for i in rss.parse_feed(FEED)}
    assert "Marshall & Blue Origin Investments" in names


def test_parse_error() -> None:
    with pytest.raises(libfec.FecParseError):
        rss.parse_feed("<rss><channel><title>x</ti")
    with pytest.raises(libfec.FecParseError):
        rss.parse_feed(b"\xff\xfe")
    with pytest.raises(TypeError):
        rss.parse_feed(123)  # type: ignore[arg-type]


def test_feed_url() -> None:
    base = "https://efilingapps.fec.gov/rss/generate"
    assert rss.feed_url() == f"{base}?preDefinedFilingType=ALL"
    assert rss.feed_url("PAC") == f"{base}?preDefinedFilingType=F3X"
    assert (
        rss.feed_url("monthly", committees=["C00505412", "C00513531"], forms="f3x")
        == f"{base}?cids=C00505412,C00513531&forms=F3X"
    )
    with pytest.raises(ValueError, match="unknown preset"):
        rss.feed_url("weekly")


@pytest.mark.network
def test_fetch_live() -> None:
    feed = rss.fetch("pac")
    assert len(feed) > 0
    assert all(i.form_type and i.form_type.startswith("F3X") for i in feed)
