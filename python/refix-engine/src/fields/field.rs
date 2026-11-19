use super::Fields;
use crate::raw;
use pyo3::exceptions::PyTypeError;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyAnyMethods, PyDict, PyDictMethods, PyString, PyTypeMethods};
use pyo3::{Bound, Py, PyAny, PyClassInitializer, PyResult, intern, pyclass, pymethods};

/// A field read as text.
#[pyclass(extends = Field, frozen, module = "refix.fields")]
pub(crate) struct Str;

#[pymethods]
impl Str {
    #[new]
    #[pyo3(signature = (tag, /))]
    fn new(tag: u32) -> PyClassInitializer<Self> {
        PyClassInitializer::from(Field::new(tag)).add_subclass(Self)
    }
}

// The base class for every field type.
#[pyclass(subclass, frozen, name = "_Field", module = "refix._core")]
pub(crate) struct Field {
    tag: u32,
    // The attribute the declaration is assigned to, which is where it caches.
    name: PyOnceLock<Py<PyString>>,
}

impl Field {
    fn new(tag: u32) -> Self {
        Self {
            tag,
            name: PyOnceLock::new(),
        }
    }
}

#[pymethods]
impl Field {
    fn __set_name__(&self, _owner: &Bound<'_, PyAny>, name: Bound<'_, PyString>) {
        let _ = self.name.set(name.py(), name.unbind());
    }

    fn __get__<'py>(
        slf: &Bound<'py, Self>,
        obj: Option<&Bound<'py, PyAny>>,
        _owner: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let Some(obj) = obj else {
            return Ok(slf.clone().into_any());
        };
        let py = slf.py();
        let field = slf.get();
        let Ok(fields) = obj.cast::<Fields>() else {
            let class = obj.get_type().qualname()?;
            return Err(PyTypeError::new_err(format!(
                "'{class}' is not a Message, Component or GroupInstance"
            )));
        };

        let value = raw::get_str(py, &fields.get().scope(), field.tag)?
            .map_or_else(|| py.None().into_bound(py), Bound::into_any);
        if let Some(name) = field.name.get(py) {
            let dict = obj
                .getattr(intern!(py, "__dict__"))?
                .cast_into::<PyDict>()?;
            dict.set_item(name, &value)?;
        }

        Ok(value)
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let name = slf.get_type().qualname()?;
        Ok(format!("{name}({})", slf.get().tag))
    }
}
