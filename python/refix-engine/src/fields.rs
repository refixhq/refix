mod attribute;
mod coded;
mod declared;
mod field;
mod group;
mod view;

pub(crate) use field::{
    Bool, Bytes, Date, Decimal, Int, MonthYear, MultipleValues, Str, Time, TimeNs, UtcTimestamp,
    UtcTimestampNs,
};
pub(crate) use group::Group;
pub(crate) use view::View;

use crate::raw::{RawMessage, Scope};
use declared::DeclaredTags;
use pyo3::exceptions::{PyAttributeError, PyTypeError};
use pyo3::sync::PyOnceLock;
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
pub(crate) struct Component {
    // The tags of the owner, when the component is a view.
    owner_tags: PyOnceLock<Py<DeclaredTags>>,
}

impl Component {
    // Makes the component end its groups at the tags of its owner.
    fn set_owner_tags(&self, py: Python<'_>, tags: Py<DeclaredTags>) {
        let _ = self.owner_tags.set(py, tags);
    }
}

#[pymethods]
impl Component {
    #[new]
    #[pyo3(signature = (raw, /))]
    fn new(raw: &Bound<'_, PyAny>) -> PyResult<PyClassInitializer<Self>> {
        let source = Source::try_from(raw)?;
        let component = Self {
            owner_tags: PyOnceLock::new(),
        };
        Ok(PyClassInitializer::from(Fields { source }).add_subclass(component))
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

    // The message the fields are part of.
    fn message(&self, py: Python<'_>) -> Py<RawMessage> {
        match &self.source {
            Source::Message(message) => message.clone_ref(py),
            Source::Scope(scope) => scope.get().message(py),
        }
    }

    // Returns the tags that end a group read from these fields. A view
    // reads the fields of its owner, so it uses the owner's.
    fn known_tags<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, DeclaredTags>> {
        let py = slf.py();
        if let Ok(component) = slf.cast::<Component>()
            && let Some(tags) = component.get().owner_tags.get(py)
        {
            return Ok(tags.bind(py).clone());
        }
        DeclaredTags::of(&slf.get_type())
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
