use super::Component;
use super::attribute::Attribute;
use pyo3::exceptions::PyTypeError;
use pyo3::types::{PyAnyMethods, PyString, PyType, PyTypeMethods};
use pyo3::{Bound, Py, PyAny, PyResult, Python, pyclass, pymethods};

/// A component read as a view of the fields of its owner.
#[pyclass(frozen, module = "refix.fields")]
pub(crate) struct View {
    component: Py<PyType>,
    attribute: Attribute,
}

impl View {
    pub(super) fn component(&self) -> &Py<PyType> {
        &self.component
    }
}

#[pymethods]
impl View {
    #[new]
    #[pyo3(signature = (component, /))]
    fn new(component: Bound<'_, PyType>) -> PyResult<Self> {
        if !component.is_subclass_of::<Component>()? {
            let name = component.qualname()?;
            return Err(PyTypeError::new_err(format!(
                "'{name}' is not a subclass of Component"
            )));
        }
        Ok(Self {
            component: component.unbind(),
            attribute: Attribute::new(),
        })
    }

    fn __set_name__(&self, _owner: &Bound<'_, PyAny>, name: Bound<'_, PyString>) {
        self.attribute.set_name(name);
    }

    fn __get__<'py>(
        slf: &Bound<'py, Self>,
        obj: Option<&Bound<'py, PyAny>>,
        _owner: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let view = slf.get();
        view.attribute.get(slf.as_any(), obj, |fields| {
            let py = fields.py();
            let raw = fields.get().raw(py);
            view.component.bind(py).call1((raw,))
        })
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let component = self.component.bind(py).qualname()?;
        Ok(format!("View({component})"))
    }
}
