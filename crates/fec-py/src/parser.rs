use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::io::Cursor;
use std::path::PathBuf;

/// Python wrapper for FilingHeader
#[pyclass]
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
}

#[pymethods]
impl Header {
    fn __repr__(&self) -> String {
        format!(
            "Header(fec_version='{}', software_name='{}', software_version='{}')",
            self.fec_version, self.software_name, self.software_version
        )
    }
}

/// Python wrapper for FilingCover
#[pyclass]
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
        let dict = PyDict::new_bound(py);
        dict.set_item("form_type", &self.form_type)?;
        dict.set_item("filer_id", &self.filer_id)?;
        dict.set_item("filer_name", &self.filer_name)?;
        dict.set_item("report_code", &self.report_code)?;
        dict.set_item("coverage_from_date", &self.coverage_from_date)?;
        dict.set_item("coverage_through_date", &self.coverage_through_date)?;
        Ok(dict)
    }
}

/// Python wrapper for FilingRow (itemization)
#[pyclass]
#[derive(Clone)]
pub struct Itemization {
    #[pyo3(get)]
    pub row_type: String,
    fields: Vec<String>,
}

#[pymethods]
impl Itemization {
    fn __repr__(&self) -> String {
        format!(
            "Itemization(row_type='{}', {} fields)",
            self.row_type,
            self.fields.len()
        )
    }

    fn __len__(&self) -> usize {
        self.fields.len()
    }

    fn __getitem__(&self, idx: isize) -> PyResult<String> {
        let len = self.fields.len() as isize;
        let actual_idx = if idx < 0 {
            (len + idx) as usize
        } else {
            idx as usize
        };

        self.fields
            .get(actual_idx)
            .cloned()
            .ok_or_else(|| pyo3::exceptions::PyIndexError::new_err("Index out of range"))
    }

    /// Get all fields as a list
    fn fields(&self) -> Vec<String> {
        self.fields.clone()
    }
}

/// Main Filing class
#[pyclass]
pub struct Filing {
    header: Header,
    cover: Cover,
    itemizations: Vec<Itemization>,
}

#[pymethods]
impl Filing {
    #[new]
    #[pyo3(signature = (source))]
    pub fn new(source: &Bound<'_, PyAny>) -> PyResult<Self> {
        // Handle different input types: path (str), bytes, or file-like object
        let (reader, source_length): (Box<dyn std::io::Read>, usize) =
            if let Ok(path_str) = source.extract::<String>() {
                // It's a file path
                let path = PathBuf::from(path_str);
                let file = std::fs::File::open(&path).map_err(|e| {
                    pyo3::exceptions::PyIOError::new_err(format!("Failed to open file: {}", e))
                })?;
                let len = file
                    .metadata()
                    .map_err(|e| {
                        pyo3::exceptions::PyIOError::new_err(format!(
                            "Failed to get file metadata: {}",
                            e
                        ))
                    })?
                    .len() as usize;
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
        let mut filing = fec_parser::Filing::from_reader(
            reader,
            "filing".to_string(),
            source_length,
        )
        .map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to parse filing: {}", e))
        })?;

        // Convert header
        let header = Header {
            record_type: filing.header.record_type.clone(),
            ef_type: filing.header.ef_type.clone(),
            fec_version: filing.header.fec_version.clone(),
            software_name: filing.header.software_name.clone(),
            software_version: filing.header.software_version.clone(),
            report_id: filing.header.report_id.clone(),
            report_number: filing.header.report_number.clone(),
            comment: filing.header.comment.clone(),
        };

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
        let mut itemizations = Vec::new();
        while let Some(row_result) = filing.next_row() {
            let row = row_result.map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Failed to read row: {}", e))
            })?;

            let fields: Vec<String> = row.record.iter().map(|s| s.to_string()).collect();
            itemizations.push(Itemization {
                row_type: row.row_type,
                fields,
            });
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
    fn itemizations(&self) -> Vec<Itemization> {
        self.itemizations.clone()
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
    let f = fec_parser::Filing::from_reader(contents, "123".to_string(), contents.len()).map_err(
        |e| pyo3::exceptions::PyValueError::new_err(format!("Failed to parse filing: {}", e)),
    )?;
    Ok(f.header.fec_version)
}

/// Parser submodule for FEC file parsing
#[pymodule]
pub fn parser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(fec_header, m)?)?;
    m.add_class::<Filing>()?;
    m.add_class::<Header>()?;
    m.add_class::<Cover>()?;
    m.add_class::<Itemization>()?;
    Ok(())
}
