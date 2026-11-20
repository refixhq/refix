use super::Fields;
use pyo3::exceptions::PyTypeError;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyAnyMethods, PyDict, PyDictMethods, PyString, PyTypeMethods};
use pyo3::{Bound, Py, PyAny, PyResult, intern};

// The attribute a declaration is assigned to, which is where it caches.
pub(super) struct Attribute {
    name: PyOnceLock<Py<PyString>>,
}

impl Attribute {
    pub(super) fn new() -> Self {
        Self {
            name: PyOnceLock::new(),
        }
    }

    // Keeps the name Python passes to `__set_name__`.
    pub(super) fn set_name(&self, name: Bound<'_, PyString>) {
        let _ = self.name.set(name.py(), name.unbind());
    }

    // Reads from the object with `read` and caches the value on it. Access
    // on the class returns `declaration`.
    pub(super) fn get<'py>(
        &self,
        declaration: &Bound<'py, PyAny>,
        obj: Option<&Bound<'py, PyAny>>,
        read: impl FnOnce(&Bound<'py, Fields>) -> PyResult<Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let Some(obj) = obj else {
            return Ok(declaration.clone());
        };
        let py = obj.py();
        let Ok(fields) = obj.cast::<Fields>() else {
            let class = obj.get_type().qualname()?;
            return Err(PyTypeError::new_err(format!(
                "'{class}' is not a Message, Component or GroupInstance"
            )));
        };

        let value = read(fields)?;
        if let Some(name) = self.name.get(py) {
            let dict = obj
                .getattr(intern!(py, "__dict__"))?
                .cast_into::<PyDict>()?;
            dict.set_item(name, &value)?;
        }

        Ok(value)
    }
}
