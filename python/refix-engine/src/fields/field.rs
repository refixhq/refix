use super::Fields;
use crate::raw;
use pyo3::exceptions::PyTypeError;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyAnyMethods, PyDict, PyDictMethods, PyString, PyTypeMethods};
use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyClassInitializer, PyResult, Python, intern, pyclass,
    pymethods,
};
use refix_message::Scope as CoreScope;

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

    fn __get__<'py>(
        slf: &Bound<'py, Self>,
        obj: Option<&Bound<'py, PyAny>>,
        _owner: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Field::get(slf.as_super(), obj, |py, scope, tag| {
            raw::get_str(py, scope, tag)?.into_bound_py_any(py)
        })
    }
}

/// A field read as an integer.
#[pyclass(extends = Field, frozen, module = "refix.fields")]
pub(crate) struct Int;

#[pymethods]
impl Int {
    #[new]
    #[pyo3(signature = (tag, /))]
    fn new(tag: u32) -> PyClassInitializer<Self> {
        PyClassInitializer::from(Field::new(tag)).add_subclass(Self)
    }

    fn __get__<'py>(
        slf: &Bound<'py, Self>,
        obj: Option<&Bound<'py, PyAny>>,
        _owner: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Field::get(slf.as_super(), obj, |py, scope, tag| {
            raw::get_int(scope, tag)?.into_bound_py_any(py)
        })
    }
}

/// A field read as a `Decimal`.
#[pyclass(extends = Field, frozen, module = "refix.fields")]
pub(crate) struct Decimal;

#[pymethods]
impl Decimal {
    #[new]
    #[pyo3(signature = (tag, /))]
    fn new(tag: u32) -> PyClassInitializer<Self> {
        PyClassInitializer::from(Field::new(tag)).add_subclass(Self)
    }

    fn __get__<'py>(
        slf: &Bound<'py, Self>,
        obj: Option<&Bound<'py, PyAny>>,
        _owner: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Field::get(slf.as_super(), obj, |py, scope, tag| {
            raw::get_decimal(py, scope, tag)?.into_bound_py_any(py)
        })
    }
}

/// A field read as a boolean.
#[pyclass(extends = Field, frozen, module = "refix.fields")]
pub(crate) struct Bool;

#[pymethods]
impl Bool {
    #[new]
    #[pyo3(signature = (tag, /))]
    fn new(tag: u32) -> PyClassInitializer<Self> {
        PyClassInitializer::from(Field::new(tag)).add_subclass(Self)
    }

    fn __get__<'py>(
        slf: &Bound<'py, Self>,
        obj: Option<&Bound<'py, PyAny>>,
        _owner: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Field::get(slf.as_super(), obj, |py, scope, tag| {
            raw::get_bool(scope, tag)?.into_bound_py_any(py)
        })
    }
}

/// A field read as bytes.
#[pyclass(extends = Field, frozen, module = "refix.fields")]
pub(crate) struct Bytes;

#[pymethods]
impl Bytes {
    #[new]
    #[pyo3(signature = (tag, /))]
    fn new(tag: u32) -> PyClassInitializer<Self> {
        PyClassInitializer::from(Field::new(tag)).add_subclass(Self)
    }

    fn __get__<'py>(
        slf: &Bound<'py, Self>,
        obj: Option<&Bound<'py, PyAny>>,
        _owner: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Field::get(slf.as_super(), obj, |py, scope, tag| {
            raw::get(py, scope, tag).into_bound_py_any(py)
        })
    }
}

/// A field read as a `date`.
#[pyclass(extends = Field, frozen, module = "refix.fields")]
pub(crate) struct Date;

#[pymethods]
impl Date {
    #[new]
    #[pyo3(signature = (tag, /))]
    fn new(tag: u32) -> PyClassInitializer<Self> {
        PyClassInitializer::from(Field::new(tag)).add_subclass(Self)
    }

    fn __get__<'py>(
        slf: &Bound<'py, Self>,
        obj: Option<&Bound<'py, PyAny>>,
        _owner: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Field::get(slf.as_super(), obj, |py, scope, tag| {
            raw::get_date(py, scope, tag)?.into_bound_py_any(py)
        })
    }
}

/// A field read as a naive `time`.
///
/// Truncates to the microsecond.
#[pyclass(extends = Field, frozen, module = "refix.fields")]
pub(crate) struct Time;

#[pymethods]
impl Time {
    #[new]
    #[pyo3(signature = (tag, /))]
    fn new(tag: u32) -> PyClassInitializer<Self> {
        PyClassInitializer::from(Field::new(tag)).add_subclass(Self)
    }

    fn __get__<'py>(
        slf: &Bound<'py, Self>,
        obj: Option<&Bound<'py, PyAny>>,
        _owner: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Field::get(slf.as_super(), obj, |py, scope, tag| {
            raw::get_time(py, scope, tag)?.into_bound_py_any(py)
        })
    }
}

/// A field read as nanoseconds since midnight.
#[pyclass(extends = Field, frozen, module = "refix.fields")]
pub(crate) struct TimeNs;

#[pymethods]
impl TimeNs {
    #[new]
    #[pyo3(signature = (tag, /))]
    fn new(tag: u32) -> PyClassInitializer<Self> {
        PyClassInitializer::from(Field::new(tag)).add_subclass(Self)
    }

    fn __get__<'py>(
        slf: &Bound<'py, Self>,
        obj: Option<&Bound<'py, PyAny>>,
        _owner: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Field::get(slf.as_super(), obj, |py, scope, tag| {
            raw::get_time_ns(scope, tag)?.into_bound_py_any(py)
        })
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

    // Reads the field with `read` and caches the value on the instance.
    fn get<'py>(
        slf: &Bound<'py, Self>,
        obj: Option<&Bound<'py, PyAny>>,
        read: impl FnOnce(Python<'py>, &CoreScope<'_>, u32) -> PyResult<Bound<'py, PyAny>>,
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

        let value = read(py, &fields.get().scope(), field.tag)?;
        if let Some(name) = field.name.get(py) {
            let dict = obj
                .getattr(intern!(py, "__dict__"))?
                .cast_into::<PyDict>()?;
            dict.set_item(name, &value)?;
        }

        Ok(value)
    }
}

#[pymethods]
impl Field {
    fn __set_name__(&self, _owner: &Bound<'_, PyAny>, name: Bound<'_, PyString>) {
        let _ = self.name.set(name.py(), name.unbind());
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let name = slf.get_type().qualname()?;
        Ok(format!("{name}({})", slf.get().tag))
    }
}
