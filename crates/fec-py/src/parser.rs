use pyo3::prelude::*;
use fec_parser::Filing;

#[pyfunction]
pub fn fec_header(contents: &[u8]) -> String {
  let f = Filing::from_reader(contents, "123".to_string(), contents.len()).unwrap();
  
   f.header.fec_version
}

/// Parser submodule for FEC file parsing
#[pymodule]
pub fn parser(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(fec_header, m)?)?;
    Ok(())
}
