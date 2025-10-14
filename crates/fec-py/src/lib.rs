use pyo3::prelude::*;
use fec_parser::Filing;

#[pyfunction]
fn hello_from_bin() -> String {
  let x = std::io::Cursor::new(include_bytes!("../../1890336.fec"));
  let f = Filing::from_reader(x, "123".to_string(), None).unwrap_or("TODO FAIL")
  
   f.header.fec_version
}

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(hello_from_bin, m)?)?;
    Ok(())
}
