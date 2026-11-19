mod field;

pub(crate) use field::{Bool, Bytes, Date, Decimal, Int, Str, Time, TimeNs, UtcTimestamp};

use crate::raw::{RawMessage, Scope};
use pyo3::exceptions::{PyAttributeError, PyTypeError};
use pyo3::types::{PyAnyMethods, PyType, PyTypeMethods};
use pyo3::{Bound, Py, PyAny, PyClassInitializer, PyErr, PyResult, Python, pyclass, pymethods};
use refix_message::Scope as CoreScope;

/// The base class of a generated message.
#[pyclass(extends = Fields, subclass, frozen, module = "refix.fields")]
pub(crate) struct Message;

#[pymethods]
impl Message {
    #[new]
    #[pyo3(signature = (raw, /))]
    fn new(raw: Bound<'_, RawMessage>) -> PyClassInitializer<Self> {
        let source = Source::Message(raw.unbind());
        PyClassInitializer::from(Fields { source }).add_subclass(Self)
    }
}

/// The base class of a generated component.
#[pyclass(extends = Fields, subclass, frozen, module = "refix.fields")]
pub(crate) struct Component;

#[pymethods]
impl Component {
    #[new]
    #[pyo3(signature = (raw, /))]
    fn new(raw: &Bound<'_, PyAny>) -> PyResult<PyClassInitializer<Self>> {
        let source = Source::try_from(raw)?;
        Ok(PyClassInitializer::from(Fields { source }).add_subclass(Self))
    }
}

/// The base class of a generated group instance.
#[pyclass(extends = Fields, subclass, frozen, module = "refix.fields")]
pub(crate) struct GroupInstance;

#[pymethods]
impl GroupInstance {
    #[new]
    #[pyo3(signature = (raw, /))]
    fn new(raw: Bound<'_, Scope>) -> PyClassInitializer<Self> {
        let source = Source::Scope(raw.unbind());
        PyClassInitializer::from(Fields { source }).add_subclass(Self)
    }
}

// The base class for messages, components and groups, which all expose fields.
#[pyclass(subclass, frozen, name = "_Fields", module = "refix._core")]
pub(crate) struct Fields {
    source: Source,
}

impl Fields {
    fn scope(&self) -> CoreScope<'_> {
        match &self.source {
            Source::Message(message) => message.get().scope(),
            Source::Scope(scope) => scope.get().scope(),
        }
    }
}

#[pymethods]
impl Fields {
    /// The message or scope the fields are read from.
    #[getter]
    fn raw(&self, py: Python<'_>) -> Py<PyAny> {
        match &self.source {
            Source::Message(message) => message.clone_ref(py).into_any(),
            Source::Scope(scope) => scope.clone_ref(py).into_any(),
        }
    }

    /// Wrap a received message or a part of one.
    #[classmethod]
    #[pyo3(signature = (raw, /))]
    fn from_raw<'py>(
        cls: &Bound<'py, PyType>,
        raw: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        cls.call1((raw,))
    }

    fn __setattr__(slf: &Bound<'_, Self>, name: &str, _value: &Bound<'_, PyAny>) -> PyResult<()> {
        immutable(slf, name)
    }

    fn __delattr__(slf: &Bound<'_, Self>, name: &str) -> PyResult<()> {
        immutable(slf, name)
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let name = slf.get_type().qualname()?;
        let raw = slf.get().raw(slf.py());
        Ok(format!("{name}({})", raw.bind(slf.py()).str()?))
    }
}

fn immutable(object: &Bound<'_, Fields>, name: &str) -> PyResult<()> {
    let class = object.get_type().qualname()?;
    Err(PyAttributeError::new_err(format!(
        "cannot change '{name}': '{class}' objects are immutable"
    )))
}

enum Source {
    Message(Py<RawMessage>),
    Scope(Py<Scope>),
}

impl TryFrom<&Bound<'_, PyAny>> for Source {
    type Error = PyErr;

    fn try_from(raw: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(message) = raw.cast::<RawMessage>() {
            Ok(Self::Message(message.clone().unbind()))
        } else if let Ok(scope) = raw.cast::<Scope>() {
            Ok(Self::Scope(scope.clone().unbind()))
        } else {
            let name = raw.get_type().qualname()?;
            Err(PyTypeError::new_err(format!(
                "'{name}' object is not a RawMessage or a Scope"
            )))
        }
    }
}
