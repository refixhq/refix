use pyo3::types::{PyBytes, PyString};
use pyo3::{Bound, PyErr, PyResult, Python, import_exception, pyclass, pymethods};
use refix_message::{InvalidValue, RawMessage as CoreRawMessage, Tag};

import_exception!(refix.errors, InvalidValueError);

#[pyclass(frozen, module = "refix._core")]
pub(crate) struct RawMessage(CoreRawMessage);

impl RawMessage {
    pub(crate) fn new(inner: CoreRawMessage) -> Self {
        Self(inner)
    }
}

#[pymethods]
impl RawMessage {
    #[getter]
    fn bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.0.bytes())
    }

    fn get<'py>(&self, py: Python<'py>, tag: u32) -> Option<Bound<'py, PyBytes>> {
        self.0.get(Tag(tag)).map(|value| PyBytes::new(py, value))
    }

    fn get_str<'py>(&self, py: Python<'py>, tag: u32) -> PyResult<Option<Bound<'py, PyString>>> {
        let value = self.0.get_str(Tag(tag)).map_err(to_py_err)?;
        Ok(value.map(|value| PyString::new(py, value)))
    }

    fn get_int(&self, tag: u32) -> PyResult<Option<i64>> {
        self.0.get_int(Tag(tag)).map_err(to_py_err)
    }

    fn entries<'py>(&self, py: Python<'py>) -> Vec<(u32, Bound<'py, PyBytes>)> {
        self.0
            .entries()
            .map(|(tag, value)| (tag.0, PyBytes::new(py, value)))
            .collect()
    }
}

fn to_py_err(error: InvalidValue) -> PyErr {
    InvalidValueError::new_err((error.tag.0,))
}
