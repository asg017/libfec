//! `libfec.rss`: the `fec-rss` crate's feed parser and URL builder. No HTTP here;
//! `python/libfec/rss.py` fetches with `urllib`.

use jiff::civil::Date;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::errors::parse_error;

/// One new filing in the feed.
#[pyclass(module = "libfec.rss", frozen, get_all)]
pub struct Item {
    /// `"New filing by <committee name>"`.
    pub title: String,
    /// The filing's `.fec` URL on docquery.fec.gov.
    pub link: String,
    /// The item's HTML description, including the `*********CommitteeId: …` trailer.
    pub description: String,
    /// `pubDate`, as an aware UTC `datetime`.
    pub pub_date: Option<jiff::Timestamp>,
    pub guid: String,
    pub committee_id: Option<String>,
    pub filing_id: Option<String>,
    pub form_type: Option<String>,
    pub coverage_from: Option<Date>,
    pub coverage_through: Option<Date>,
    pub report_type: Option<String>,
    /// The title without its `"New filing by "` prefix.
    pub committee_name: String,
}

/// `MM/DD/YYYY` (the feed's coverage dates) to a date.
fn mdy(s: &Option<String>) -> Option<Date> {
    let mut parts = s.as_deref()?.splitn(3, '/');
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    let year = parts.next()?.parse().ok()?;
    Date::new(year, month, day).ok()
}

impl From<fec_rss::Item> for Item {
    fn from(item: fec_rss::Item) -> Self {
        Item {
            committee_name: item.extract_committee_name().to_owned(),
            coverage_from: mdy(&item.coverage_from),
            coverage_through: mdy(&item.coverage_through),
            title: item.title,
            link: item.link,
            description: item.description,
            pub_date: item.pub_date,
            guid: item.guid,
            committee_id: item.committee_id,
            filing_id: item.filing_id,
            form_type: item.form_type,
            report_type: item.report_type,
        }
    }
}

#[pymethods]
impl Item {
    fn __repr__(&self) -> String {
        format!(
            "Item(filing_id={:?}, form_type={:?}, committee_name={:?})",
            self.filing_id.as_deref().unwrap_or(""),
            self.form_type.as_deref().unwrap_or(""),
            self.committee_name
        )
    }

    /// Every field as a plain `dict`.
    fn to_dict<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyDict>> {
        let py = slf.py();
        let i = slf.get();
        let d = PyDict::new(py);
        d.set_item("title", &i.title)?;
        d.set_item("link", &i.link)?;
        d.set_item("description", &i.description)?;
        d.set_item("pub_date", i.pub_date)?;
        d.set_item("guid", &i.guid)?;
        d.set_item("committee_id", &i.committee_id)?;
        d.set_item("filing_id", &i.filing_id)?;
        d.set_item("form_type", &i.form_type)?;
        d.set_item("coverage_from", i.coverage_from)?;
        d.set_item("coverage_through", i.coverage_through)?;
        d.set_item("report_type", &i.report_type)?;
        d.set_item("committee_name", &i.committee_name)?;
        Ok(d)
    }
}

/// A parsed feed: channel metadata plus its items, newest first.
#[pyclass(module = "libfec.rss", frozen, get_all)]
pub struct Feed {
    pub title: String,
    pub link: String,
    pub description: String,
    pub items: Py<PyList>,
}

#[pymethods]
impl Feed {
    fn __repr__(&self, py: Python<'_>) -> String {
        format!(
            "Feed(title={:?}, items={})",
            self.title,
            self.items.bind(py).len()
        )
    }

    fn __len__(&self, py: Python<'_>) -> usize {
        self.items.bind(py).len()
    }

    fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.items.bind(py).try_iter().map(Bound::into_any)
    }
}

/// Parse the feed's XML (`str`, or UTF-8 `bytes`).
#[pyfunction]
pub fn parse_feed(py: Python<'_>, xml: &Bound<'_, PyAny>) -> PyResult<Feed> {
    let text: std::borrow::Cow<'_, str> = if let Ok(s) = xml.extract::<&str>() {
        s.into()
    } else if let Ok(b) = xml.extract::<&[u8]>() {
        std::str::from_utf8(b).map_err(parse_error)?.into()
    } else {
        return Err(pyo3::exceptions::PyTypeError::new_err(
            "parse_feed() takes str or bytes",
        ));
    };
    let feed = py
        .detach(|| fec_rss::parse_feed(&text))
        .map_err(parse_error)?;
    let items = PyList::empty(py);
    for item in feed.items {
        items.append(Py::new(py, Item::from(item))?)?;
    }
    Ok(Feed {
        title: feed.title,
        link: feed.link,
        description: feed.description,
        items: items.unbind(),
    })
}

/// The feed URL for a preset (`"all"`, `"monthly"`, `"quarterly"`, `"presidential"`,
/// `"congressional"`, `"pac"`) or custom filters (comma-separated values). Any
/// custom filter overrides `preset`.
#[pyfunction]
#[pyo3(signature = (preset = "all", *, committees = None, forms = None, states = None, parties = None))]
pub fn feed_url(
    preset: &str,
    committees: Option<String>,
    forms: Option<String>,
    states: Option<String>,
    parties: Option<String>,
) -> PyResult<String> {
    use fec_rss::Preset;
    let preset = match preset.to_ascii_lowercase().as_str() {
        "all" => Preset::All,
        "monthly" => Preset::Monthly,
        "quarterly" => Preset::Quarterly,
        "presidential" => Preset::Presidential,
        "congressional" => Preset::Congressional,
        "pac" => Preset::Pac,
        other => {
            return Err(PyValueError::new_err(format!(
                "unknown preset {other:?}: expected all, monthly, quarterly, presidential, congressional or pac"
            )))
        }
    };
    Ok(fec_rss::FeedQuery {
        preset,
        committees,
        forms,
        states,
        parties,
    }
    .url())
}
