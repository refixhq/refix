use super::Fields;
use super::coded::Coded;
use crate::raw;
use pyo3::exceptions::PyTypeError;
use pyo3::sync::PyOnceLock;
use pyo3::types::{
    PyAnyMethods, PyDict, PyDictMethods, PyString, PyTuple, PyTupleMethods, PyType, PyTypeMethods,
};
use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyClassInitializer, PyResult, Python, intern, pyclass,
    pymethods,
};
use refix_message::Scope as CoreScope;

/// A field read as text, or as a member of an enum.
#[pyclass(extends = Field, frozen, module = "refix.fields")]
pub(crate) struct Str {
    coded: Option<Coded>,
}

#[pymethods]
impl Str {
    #[new]
    #[pyo3(signature = (tag, r#enum = None, /))]
    fn new(tag: u32, r#enum: Option<Bound<'_, PyType>>) -> PyResult<PyClassInitializer<Self>> {
        let coded = r#enum.map(Coded::text).transpose()?;
        Ok(PyClassInitializer::from(Field::new(tag)).add_subclass(Self { coded }))
    }

    fn __get__<'py>(
        slf: &Bound<'py, Self>,
        obj: Option<&Bound<'py, PyAny>>,
        _owner: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let coded = slf.get().coded.as_ref();
        Field::get(slf.as_super(), obj, |py, scope, tag| {
            match (raw::get_str(py, scope, tag)?, coded) {
                (Some(value), Some(coded)) => coded.member(value.into_any()),
                (value, _) => value.into_bound_py_any(py),
            }
        })
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        Field::repr(slf.as_super(), slf.get().coded.as_ref())
    }
}

/// A field read as an integer, or as a member of an enum.
#[pyclass(extends = Field, frozen, module = "refix.fields")]
pub(crate) struct Int {
    coded: Option<Coded>,
}

#[pymethods]
impl Int {
    #[new]
    #[pyo3(signature = (tag, r#enum = None, /))]
    fn new(tag: u32, r#enum: Option<Bound<'_, PyType>>) -> PyResult<PyClassInitializer<Self>> {
        let coded = r#enum.map(Coded::integer).transpose()?;
        Ok(PyClassInitializer::from(Field::new(tag)).add_subclass(Self { coded }))
    }

    fn __get__<'py>(
        slf: &Bound<'py, Self>,
        obj: Option<&Bound<'py, PyAny>>,
        _owner: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let coded = slf.get().coded.as_ref();
        Field::get(slf.as_super(), obj, |py, scope, tag| {
            match (raw::get_int(scope, tag)?, coded) {
                (Some(value), Some(coded)) => coded.member(value.into_bound_py_any(py)?),
                (value, _) => value.into_bound_py_any(py),
            }
        })
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        Field::repr(slf.as_super(), slf.get().coded.as_ref())
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

/// A field read as a UTC `datetime`.
///
/// Truncates to the microsecond.
#[pyclass(extends = Field, frozen, module = "refix.fields")]
pub(crate) struct UtcTimestamp;

#[pymethods]
impl UtcTimestamp {
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
            raw::get_utc_timestamp(py, scope, tag)?.into_bound_py_any(py)
        })
    }
}

/// A field read as nanoseconds since the Unix epoch.
#[pyclass(extends = Field, frozen, module = "refix.fields")]
pub(crate) struct UtcTimestampNs;

#[pymethods]
impl UtcTimestampNs {
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
            raw::get_utc_timestamp_ns(scope, tag)?.into_bound_py_any(py)
        })
    }
}

/// A field read as a `MonthYear`.
#[pyclass(extends = Field, frozen, module = "refix.fields")]
pub(crate) struct MonthYear;

#[pymethods]
impl MonthYear {
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
            raw::get_month_year(py, scope, tag)?.into_bound_py_any(py)
        })
    }
}

/// A field read as a tuple of text values, or of members of an enum.
#[pyclass(extends = Field, frozen, module = "refix.fields")]
pub(crate) struct MultipleValues {
    coded: Option<Coded>,
}

#[pymethods]
impl MultipleValues {
    #[new]
    #[pyo3(signature = (tag, r#enum = None, /))]
    fn new(tag: u32, r#enum: Option<Bound<'_, PyType>>) -> PyResult<PyClassInitializer<Self>> {
        let coded = r#enum.map(Coded::text).transpose()?;
        Ok(PyClassInitializer::from(Field::new(tag)).add_subclass(Self { coded }))
    }

    fn __get__<'py>(
        slf: &Bound<'py, Self>,
        obj: Option<&Bound<'py, PyAny>>,
        _owner: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let coded = slf.get().coded.as_ref();
        Field::get(slf.as_super(), obj, |py, scope, tag| {
            match (raw::get_multiple_values(py, scope, tag)?, coded) {
                (Some(values), Some(coded)) => {
                    let members: Vec<_> = values
                        .iter()
                        .map(|value| coded.member(value))
                        .collect::<PyResult<_>>()?;
                    PyTuple::new(py, members).map(Bound::into_any)
                }
                (values, _) => values.into_bound_py_any(py),
            }
        })
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        Field::repr(slf.as_super(), slf.get().coded.as_ref())
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
    pub(super) fn new(tag: u32) -> Self {
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
        Self::get_from(slf, obj, |fields, tag| {
            read(fields.py(), &fields.get().scope(), tag)
        })
    }

    pub(super) fn tag(&self) -> u32 {
        self.tag
    }

    // Reads from the object with `read` and caches the value on it.
    fn get_from<'py>(
        slf: &Bound<'py, Self>,
        obj: Option<&Bound<'py, PyAny>>,
        read: impl FnOnce(&Bound<'py, Fields>, u32) -> PyResult<Bound<'py, PyAny>>,
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

        let value = read(fields, field.tag)?;
        if let Some(name) = field.name.get(py) {
            let dict = obj
                .getattr(intern!(py, "__dict__"))?
                .cast_into::<PyDict>()?;
            dict.set_item(name, &value)?;
        }

        Ok(value)
    }

    // Writes the declaration as it is constructed, such as `Str(54, Side)`.
    fn repr(slf: &Bound<'_, Self>, coded: Option<&Coded>) -> PyResult<String> {
        let name = slf.get_type().qualname()?;
        let tag = slf.get().tag;
        match coded {
            Some(coded) => Ok(format!("{name}({tag}, {})", coded.name(slf.py())?)),
            None => Ok(format!("{name}({tag})")),
        }
    }
}

#[pymethods]
impl Field {
    fn __set_name__(&self, _owner: &Bound<'_, PyAny>, name: Bound<'_, PyString>) {
        let _ = self.name.set(name.py(), name.unbind());
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        Self::repr(slf, None)
    }
}
