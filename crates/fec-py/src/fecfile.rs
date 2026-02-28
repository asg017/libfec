use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;
use std::io::Cursor;

/// Parse FEC document content and return structured data
///
/// Compatible with fecfile.loads() API
#[pyfunction]
#[pyo3(signature = (input, options=None))]
pub fn loads<'py>(
    py: Python<'py>,
    input: &Bound<'_, PyAny>,
    options: Option<&Bound<'py, PyDict>>,
) -> PyResult<Bound<'py, PyDict>> {
    // Extract options
    let filter_itemizations: Option<Vec<String>> = options
        .and_then(|opts| opts.get_item("filter_itemizations").ok().flatten())
        .and_then(|v| v.extract().ok());

    let _as_strings: bool = options
        .and_then(|opts| opts.get_item("as_strings").ok().flatten())
        .and_then(|v| v.extract().ok())
        .unwrap_or(false);

    // Convert input to bytes
    let bytes: Vec<u8> = if let Ok(s) = input.extract::<String>() {
        s.into_bytes()
    } else if let Ok(lines) = input.extract::<Vec<String>>() {
        lines.join("\n").into_bytes()
    } else if let Ok(bytes) = input.extract::<Vec<u8>>() {
        bytes
    } else {
        return Err(pyo3::exceptions::PyTypeError::new_err(
            "Input must be a string, list of strings, or bytes",
        ));
    };

    // Parse the filing
    let mut filing =
        fec_parser::Filing::from_reader(Cursor::new(&bytes), "filing".to_string(), bytes.len())
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Failed to parse filing: {}", e))
            })?;

    // Build result dictionary
    let result = PyDict::new_bound(py);

    // Add header
    let header_dict = PyDict::new_bound(py);
    header_dict.set_item("record_type", &filing.header.record_type)?;
    header_dict.set_item("ef_type", &filing.header.ef_type)?;
    header_dict.set_item("fec_version", &filing.header.fec_version)?;
    header_dict.set_item("software_name", &filing.header.software_name)?;
    header_dict.set_item("software_version", &filing.header.software_version)?;
    if let Some(ref report_id) = filing.header.report_id {
        header_dict.set_item("report_id", report_id)?;
    }
    if let Some(ref report_number) = filing.header.report_number {
        header_dict.set_item("report_number", report_number)?;
    }
    if let Some(ref comment) = filing.header.comment {
        header_dict.set_item("comment", comment)?;
    }
    result.set_item("header", header_dict)?;

    // Add filing (cover)
    let filing_dict = PyDict::new_bound(py);
    filing_dict.set_item("form_type", &filing.cover.form_type)?;
    filing_dict.set_item("filer_committee_id_number", &filing.cover.filer_id)?;

    // Add all cover fields
    for (key, value) in &filing.cover.cover_record_kv {
        filing_dict.set_item(key.as_str(), value)?;
    }
    result.set_item("filing", filing_dict)?;

    // Add itemizations - grouped by schedule type
    let mut itemizations: HashMap<String, Vec<Bound<PyDict>>> = HashMap::new();
    let text_list = PyList::empty_bound(py);

    // Process all rows
    while let Some(row_result) = filing.next_row() {
        let row = row_result.map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to read row: {}", e))
        })?;

        let row_type = &row.row_type;

        // Apply filter if provided
        if let Some(ref filter) = filter_itemizations {
            if !filter.is_empty() {
                let matches = filter.iter().any(|prefix| row_type.starts_with(prefix));
                if !matches {
                    continue;
                }
            } else {
                // Empty filter means only header and filing
                continue;
            }
        }

        // Handle TEXT records specially
        if row_type == "TEXT" {
            let text_dict = PyDict::new_bound(py);
            text_dict.set_item("form_type", row_type)?;
            for (i, field) in row.record.iter().enumerate() {
                text_dict.set_item(format!("field_{}", i), field)?;
            }
            text_list.append(text_dict)?;
            continue;
        }

        // Determine schedule key (e.g., "SA11AI" -> "Schedule A")
        let schedule_key = if row_type.starts_with('S') && row_type.len() > 1 {
            let schedule_letter = row_type.chars().nth(1).unwrap_or(' ');
            format!("Schedule {}", schedule_letter.to_uppercase())
        } else {
            row_type.to_string()
        };

        // Get column names for this form type
        let column_names: Vec<String> = match fec_parser::mappings::column_names_for_field(
            row_type,
            &filing.header.fec_version,
        ) {
            Ok(names) => names.to_vec(),
            Err(_) => {
                // If no mapping found, use generic field names
                (0..row.record.len())
                    .map(|i| format!("field_{}", i))
                    .collect::<Vec<_>>()
            }
        };

        // Build row dictionary
        let row_dict = PyDict::new_bound(py);
        for (column_name, field) in column_names.iter().zip(row.record.iter()) {
            // For now, always return as strings (type conversion would require types.json equivalent)
            row_dict.set_item(column_name.as_str(), field)?;
        }

        // Add to appropriate schedule list
        itemizations.entry(schedule_key).or_default().push(row_dict);
    }

    // Convert itemizations map to dictionary
    let itemizations_dict = PyDict::new_bound(py);
    for (schedule, rows) in itemizations {
        let rows_list = PyList::new_bound(py, rows);
        itemizations_dict.set_item(schedule, rows_list)?;
    }
    result.set_item("itemizations", itemizations_dict)?;

    // Add text records
    result.set_item("text", text_list)?;

    Ok(result)
}

/// Parse header from FEC document
#[pyfunction]
#[pyo3(signature = (hdr))]
pub fn parse_header<'py>(
    py: Python<'py>,
    hdr: &Bound<'_, PyAny>,
) -> PyResult<(Bound<'py, PyDict>, String, usize)> {
    // Convert input to string
    let hdr_str: String = if let Ok(s) = hdr.extract::<String>() {
        s
    } else if let Ok(lines) = hdr.extract::<Vec<String>>() {
        lines.join("\n")
    } else {
        return Err(pyo3::exceptions::PyTypeError::new_err(
            "Header must be a string or list of strings",
        ));
    };

    // Parse just the header line as CSV with ASCII 28 delimiter
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b"\x1c"[0])
        .flexible(true)
        .has_headers(false)
        .from_reader(hdr_str.as_bytes());

    let record = rdr
        .records()
        .next()
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Empty header"))?
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("CSV parse error: {}", e)))?;

    // Build header dict
    let header_dict = PyDict::new_bound(py);

    let record_type = record.get(0).unwrap_or("HDR");
    let ef_type = record.get(1).unwrap_or("");
    let fec_version = record.get(2).unwrap_or("");
    let software_name = record.get(3).unwrap_or("");
    let software_version = record.get(4).unwrap_or("");

    header_dict.set_item("record_type", record_type)?;
    header_dict.set_item("ef_type", ef_type)?;
    header_dict.set_item("fec_version", fec_version)?;
    header_dict.set_item("software_name", software_name)?;
    header_dict.set_item("software_version", software_version)?;

    if let Some(report_id) = record.get(5) {
        if !report_id.trim().is_empty() {
            header_dict.set_item("report_id", report_id)?;
        }
    }
    if let Some(report_number) = record.get(6) {
        if !report_number.trim().is_empty() {
            header_dict.set_item("report_number", report_number)?;
        }
    }
    if let Some(comment) = record.get(7) {
        if !comment.trim().is_empty() {
            header_dict.set_item("comment", comment)?;
        }
    }

    let version = fec_version.to_string();
    let lines_consumed = 1; // In ASCII 28 format, header is always 1 line

    Ok((header_dict, version, lines_consumed))
}

/// Parse a single line from FEC document
#[pyfunction]
#[pyo3(signature = (line, version, _line_num=None))]
pub fn parse_line<'py>(
    py: Python<'py>,
    line: String,
    version: String,
    _line_num: Option<usize>,
) -> PyResult<Bound<'py, PyDict>> {
    // Parse the line as CSV with ASCII 28 delimiter
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b"\x1c"[0])
        .flexible(true)
        .has_headers(false)
        .from_reader(line.as_bytes());

    let record = rdr
        .records()
        .next()
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Empty line"))?
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("CSV parse error: {}", e)))?;

    let form_type = record
        .get(0)
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Line has no fields"))?;

    // Get column names for this form type
    let column_names: Vec<String> =
        match fec_parser::mappings::column_names_for_field(form_type, &version) {
            Ok(names) => names.to_vec(),
            Err(_) => {
                // If no mapping found, use generic field names
                (0..record.len())
                    .map(|i| format!("field_{}", i))
                    .collect::<Vec<_>>()
            }
        };

    // Build dictionary
    let result = PyDict::new_bound(py);
    for (column_name, field) in column_names.iter().zip(record.iter()) {
        result.set_item(column_name.as_str(), field)?;
    }

    Ok(result)
}

/// Load FEC filing from HTTP
#[pyfunction]
#[pyo3(signature = (file_number, options=None))]
pub fn from_http<'py>(
    py: Python<'py>,
    file_number: &Bound<'_, PyAny>,
    options: Option<&Bound<'py, PyDict>>,
) -> PyResult<Option<Bound<'py, PyDict>>> {
    let file_num_str = if let Ok(num) = file_number.extract::<i64>() {
        num.to_string()
    } else {
        file_number.extract::<String>()?
    };

    // Try primary URL first
    let primary_url = format!("https://docquery.fec.gov/dcdev/posted/{}.fec", file_num_str);

    // Use Python's requests library
    let requests = py.import_bound("urllib.request")?;
    let urlopen = requests.getattr("urlopen")?;

    // Try primary URL
    match urlopen.call1((primary_url,)) {
        Ok(response) => {
            let data = response.call_method0("read")?;
            let bytes: Vec<u8> = data.extract()?;
            let input = PyList::new_bound(py, [bytes]);
            let result = loads(py, input.as_any(), options)?;
            Ok(Some(result))
        }
        Err(_) => {
            // Try fallback URL
            let fallback_url =
                format!("https://docquery.fec.gov/paper/posted/{}.fec", file_num_str);
            match urlopen.call1((fallback_url,)) {
                Ok(response) => {
                    let data = response.call_method0("read")?;
                    let bytes: Vec<u8> = data.extract()?;
                    let input = PyList::new_bound(py, [bytes]);
                    let result = loads(py, input.as_any(), options)?;
                    Ok(Some(result))
                }
                Err(_) => Ok(None), // 404 - return None
            }
        }
    }
}

/// Load FEC filing from file
#[pyfunction]
#[pyo3(signature = (file_path, options=None))]
pub fn from_file<'py>(
    py: Python<'py>,
    file_path: String,
    options: Option<&Bound<'py, PyDict>>,
) -> PyResult<Bound<'py, PyDict>> {
    // Read file
    let bytes = std::fs::read(&file_path)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(format!("Failed to read file: {}", e)))?;

    loads(py, bytes.into_py(py).bind(py), options)
}

/// Print example output showing first itemization of each type
#[pyfunction]
pub fn print_example(parsed: &Bound<'_, PyDict>) -> PyResult<()> {
    let itemizations_item = parsed
        .get_item("itemizations")?
        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("itemizations"))?;
    let itemizations: &Bound<PyDict> = itemizations_item.downcast()?;

    let example = PyDict::new_bound(parsed.py());
    example.set_item("header", parsed.get_item("header")?)?;
    example.set_item("filing", parsed.get_item("filing")?)?;

    let example_itemizations = PyDict::new_bound(parsed.py());

    for item in itemizations.items() {
        let (key, value): (Bound<PyAny>, Bound<PyAny>) = item.extract()?;
        let list: &Bound<PyList> = value.downcast()?;
        if list.len() > 0 {
            let first_item = list.get_item(0)?;
            let single_item_list = PyList::new_bound(parsed.py(), [first_item]);
            example_itemizations.set_item(&key, single_item_list)?;
        }
    }

    example.set_item("itemizations", example_itemizations)?;
    example.set_item("text", parsed.get_item("text")?)?;

    // Print as JSON
    let json = parsed.py().import_bound("json")?;
    let json_str = json.call_method1("dumps", (example,))?;
    println!("{}", json_str);

    Ok(())
}

/// FecFile compatibility submodule
#[pymodule]
pub fn fecfile(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(loads, m)?)?;
    m.add_function(wrap_pyfunction!(parse_header, m)?)?;
    m.add_function(wrap_pyfunction!(parse_line, m)?)?;
    m.add_function(wrap_pyfunction!(from_http, m)?)?;
    m.add_function(wrap_pyfunction!(from_file, m)?)?;
    m.add_function(wrap_pyfunction!(print_example, m)?)?;
    Ok(())
}
