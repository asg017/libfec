"""The FEC e-filing RSS feed: new filings, about a week's worth, newest first.

Parsing is the ``fec-rss`` Rust crate (the same parser ``libfec rss`` uses);
:func:`fetch` is a thin ``urllib`` wrapper around it::

    from libfec import rss

    for item in rss.fetch(forms=["F3X"], states="CA"):
        print(item.filing_id, item.committee_name, item.pub_date)

    feed = rss.parse_feed(xml_bytes)   # bring your own HTTP client
"""

import urllib.request
from collections.abc import Iterable

from ._native import rss as _rss

Feed = _rss.Feed
Item = _rss.Item
parse_feed = _rss.parse_feed

_TIMEOUT = 60.0


def _join(value: str | Iterable[str] | None) -> str | None:
    if value is None or isinstance(value, str):
        return value
    return ",".join(value)


def feed_url(
    preset: str = "all",
    *,
    committees: str | Iterable[str] | None = None,
    forms: str | Iterable[str] | None = None,
    states: str | Iterable[str] | None = None,
    parties: str | Iterable[str] | None = None,
) -> str:
    """The feed URL for a preset or custom filters.

    ``preset`` is one of ``"all"``, ``"monthly"``, ``"quarterly"``,
    ``"presidential"``, ``"congressional"``, ``"pac"``. Any of ``committees``,
    ``forms``, ``states`` or ``parties`` (a string or an iterable of strings)
    switches to the feed's custom-filter format, which ignores ``preset``.
    """
    return _rss.feed_url(
        preset,
        committees=_join(committees),
        forms=_join(forms),
        states=_join(states),
        parties=_join(parties),
    )


def fetch(
    preset: str = "all",
    *,
    committees: str | Iterable[str] | None = None,
    forms: str | Iterable[str] | None = None,
    states: str | Iterable[str] | None = None,
    parties: str | Iterable[str] | None = None,
    timeout: float = _TIMEOUT,
) -> Feed:
    """Download and parse the feed (see :func:`feed_url` for the filters).

    Network errors propagate as ``urllib.error`` exceptions.
    """
    url = feed_url(
        preset, committees=committees, forms=forms, states=states, parties=parties
    )
    with urllib.request.urlopen(url, timeout=timeout) as response:
        return parse_feed(response.read())


__all__ = ["Feed", "Item", "feed_url", "fetch", "parse_feed"]
