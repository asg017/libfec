use crate::errors::{io_error, missing_mapping, parse_error};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::io::Cursor;
use std::path::PathBuf;

use crate::row::{schema_for, Row};

/// Python wrapper for FilingHeader
#[pyclass(module = "libfec_parser.parser", skip_from_py_object)]
#[derive(Clone)]
pub struct Header {
    #[pyo3(get)]
    pub record_type: String,
    #[pyo3(get)]
    pub ef_type: String,
    #[pyo3(get)]
    pub fec_version: String,
    #[pyo3(get)]
    pub software_name: String,
    #[pyo3(get)]
    pub software_version: String,
    #[pyo3(get)]
    pub report_id: Option<String>,
    #[pyo3(get)]
    pub report_number: Option<String>,
    #[pyo3(get)]
    pub comment: Option<String>,
    /// How the header is written: `"hdr"`, `"legacy_block"` (electronic
    /// 1.x/2.x `/* Header` block) or `"paper"`.
    #[pyo3(get)]
    pub style: String,
    /// Body field delimiter: `"fs"` (0x1C) or `"comma"`.
    #[pyo3(get)]
    pub delimiter: String,
    /// Sub-delimiter of combined name fields (3.x–5.x and 1.x/2.x only).
    #[pyo3(get)]
    pub name_delimiter: Option<String>,
    /// Paper filings: FEC data-entry batch number.
    #[pyo3(get)]
    pub batch_number: Option<String>,
    /// Paper filings P2.6+: date the FEC received the filing.
    #[pyo3(get)]
    pub received_date: Option<String>,
    /// Whether this is FEC data entry of a paper filing.
    #[pyo3(get)]
    pub is_paper: bool,
    legacy_fields: Vec<(String, String)>,
    schedule_counts: Vec<(String, String)>,
}

impl From<&fec_parser::FilingHeader> for Header {
    fn from(h: &fec_parser::FilingHeader) -> Self {
        Header {
            record_type: h.record_type.clone(),
            ef_type: h.ef_type.clone(),
            fec_version: h.fec_version.clone(),
            software_name: h.software_name.clone(),
            software_version: h.software_version.clone(),
            report_id: h.report_id.clone(),
            report_number: h.report_number.clone(),
            comment: h.comment.clone(),
            style: h.style.as_str().to_owned(),
            delimiter: h.delimiter.as_str().to_owned(),
            name_delimiter: h.name_delimiter.clone(),
            batch_number: h.batch_number.clone(),
            received_date: h.received_date.clone(),
            is_paper: h.is_paper(),
            legacy_fields: pairs(&h.legacy_fields),
            schedule_counts: pairs(&h.schedule_counts),
        }
    }
}

fn pairs<'a>(m: impl IntoIterator<Item = (&'a String, &'a String)>) -> Vec<(String, String)> {
    m.into_iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}

fn pairs_to_dict<'py>(py: Python<'py>, pairs: &[(String, String)]) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    for (k, v) in pairs {
        dict.set_item(k, v)?;
    }
    Ok(dict)
}

#[pymethods]
impl Header {
    fn __repr__(&self) -> String {
        format!(
            "Header(fec_version='{}', software_name='{}', software_version='{}')",
            self.fec_version, self.software_name, self.software_version
        )
    }

    /// `/* Header` block (electronic 1.x/2.x): every `key = value` line before
    /// `Schedule_Counts:`, keys as written, in file order. Empty otherwise.
    #[getter]
    fn legacy_fields<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        pairs_to_dict(py, &self.legacy_fields)
    }

    /// `/* Header` block: the `Schedule_Counts:` lines (row type -> count as
    /// written). Empty otherwise.
    #[getter]
    fn schedule_counts<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        pairs_to_dict(py, &self.schedule_counts)
    }
}

/// Python wrapper for FilingCover
#[pyclass(module = "libfec_parser.parser", skip_from_py_object)]
#[derive(Clone)]
pub struct Cover {
    #[pyo3(get)]
    pub form_type: String,
    #[pyo3(get)]
    pub filer_id: String,
    #[pyo3(get)]
    pub filer_name: String,
    #[pyo3(get)]
    pub report_code: Option<String>,
    #[pyo3(get)]
    pub coverage_from_date: Option<String>,
    #[pyo3(get)]
    pub coverage_through_date: Option<String>,
}

#[pymethods]
impl Cover {
    fn __repr__(&self) -> String {
        format!(
            "Cover(form_type='{}', filer_id='{}', filer_name='{}')",
            self.form_type, self.filer_id, self.filer_name
        )
    }

    /// Get all cover record fields as a dictionary
    fn fields<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("form_type", &self.form_type)?;
        dict.set_item("filer_id", &self.filer_id)?;
        dict.set_item("filer_name", &self.filer_name)?;
        dict.set_item("report_code", &self.report_code)?;
        dict.set_item("coverage_from_date", &self.coverage_from_date)?;
        dict.set_item("coverage_through_date", &self.coverage_through_date)?;
        Ok(dict)
    }
}

/// Main Filing class
#[pyclass(module = "libfec_parser.parser")]
pub struct Filing {
    header: Header,
    cover: Cover,
    itemizations: Vec<Py<Row>>,
}

#[pymethods]
impl Filing {
    #[new]
    #[pyo3(signature = (source))]
    pub fn new(py: Python<'_>, source: &Bound<'_, PyAny>) -> PyResult<Self> {
        // Handle different input types: path (str), bytes, or file-like object
        let (reader, source_length): (Box<dyn std::io::Read>, usize) =
            if let Ok(path_str) = source.extract::<String>() {
                // It's a file path
                let path = PathBuf::from(path_str);
                let file = std::fs::File::open(&path).map_err(|e| io_error(e, &path))?;
                let len = file.metadata().map_err(|e| io_error(e, &path))?.len() as usize;
                (Box::new(file), len)
            } else if let Ok(bytes) = source.extract::<Vec<u8>>() {
                // It's bytes
                let len = bytes.len();
                (Box::new(Cursor::new(bytes)), len)
            } else if let Ok(bytes_like) = source.call_method0("read") {
                // It's a file-like object with read() method
                let bytes: Vec<u8> = bytes_like.extract()?;
                let len = bytes.len();
                (Box::new(Cursor::new(bytes)), len)
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                "Source must be a file path (str), bytes, or file-like object with read() method"
            ));
            };

        // Parse the filing
        let mut filing =
            fec_parser::Filing::from_reader(reader, "filing".to_string(), source_length)
                .map_err(parse_error)?;

        let header = Header::from(&filing.header);

        // Convert cover
        let cover = Cover {
            form_type: filing.cover.form_type.clone(),
            filer_id: filing.cover.filer_id.clone(),
            filer_name: filing.cover.filer_name.clone(),
            report_code: filing.cover.report_code.clone(),
            coverage_from_date: filing.cover.coverage_from_date.map(|d| d.to_string()),
            coverage_through_date: filing.cover.coverage_through_date.map(|d| d.to_string()),
        };

        // Collect all itemizations
        let version = header.fec_version.clone();
        let mut itemizations = Vec::new();
        while let Some(row_result) = filing.next_row() {
            let row = row_result.map_err(parse_error)?;

            let schema = schema_for(py, &row.row_type, &version)
                .ok_or_else(|| missing_mapping(py, &row.row_type, &version, row.line))?;
            itemizations.push(Py::new(py, Row::new(schema, row.record, row.line))?);
        }

        Ok(Filing {
            header,
            cover,
            itemizations,
        })
    }

    #[getter]
    fn header(&self) -> Header {
        self.header.clone()
    }

    #[getter]
    fn cover(&self) -> Cover {
        self.cover.clone()
    }

    #[getter]
    fn itemizations(&self, py: Python<'_>) -> Vec<Py<Row>> {
        self.itemizations.iter().map(|r| r.clone_ref(py)).collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "Filing(form_type='{}', filer_id='{}', {} itemizations)",
            self.cover.form_type,
            self.cover.filer_id,
            self.itemizations.len()
        )
    }
}

#[pyfunction]
pub fn fec_header(contents: &[u8]) -> PyResult<String> {
    let f = fec_parser::Filing::from_reader(contents, "123".to_string(), contents.len())
        .map_err(parse_error)?;
    Ok(f.header.fec_version)
}
