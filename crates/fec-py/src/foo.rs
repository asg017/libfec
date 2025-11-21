use pyo3::prelude::*;

#[pyfunction]
pub fn bar() -> i32 {
    42
}

/// Foo submodule - example module
#[pymodule]
pub fn foo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(bar, m)?)?;
    Ok(())
}
