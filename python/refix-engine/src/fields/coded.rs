use pyo3::exceptions::PyTypeError;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyAnyMethods, PyDict, PyDictMethods, PyString, PyType, PyTypeMethods};
use pyo3::{Bound, Py, PyAny, PyResult, Python, intern};

/// Python's `enum.StrEnum`, imported on first use.
static STR_ENUM: PyOnceLock<Py<PyType>> = PyOnceLock::new();

/// `refix.enums.Unrecognized`, imported on first use.
static UNRECOGNIZED: PyOnceLock<Py<PyType>> = PyOnceLock::new();

// The enum of a coded field, with its members by value.
pub(super) struct Coded {
    class: Py<PyType>,
    // Built on the first read, so defining a message class does not walk the enum.
    members: PyOnceLock<Py<PyDict>>,
}

impl Coded {
    // Takes the enum of a field read as text.
    pub(super) fn text(class: Bound<'_, PyType>) -> PyResult<Self> {
        let base = STR_ENUM.import(class.py(), "enum", "StrEnum")?;
        Self::new(class, base)
    }

    fn new(class: Bound<'_, PyType>, base: &Bound<'_, PyType>) -> PyResult<Self> {
        if !class.is_subclass(base)? {
            return Err(PyTypeError::new_err(format!(
                "'{}' is not a subclass of {}",
                class.qualname()?,
                base.name()?
            )));
        }

        Ok(Self {
            class: class.unbind(),
            members: PyOnceLock::new(),
        })
    }

    // Returns the member with `value`, or `Unrecognized(value)` when there is none.
    pub(super) fn member<'py>(&self, value: Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let py = value.py();
        let members = self.members.get_or_try_init(py, || self.members(py))?;
        match members.bind(py).get_item(&value)? {
            Some(member) => Ok(member),
            None => UNRECOGNIZED
                .import(py, "refix.enums", "Unrecognized")?
                .call1((value,)),
        }
    }

    pub(super) fn name<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        self.class.bind(py).qualname()
    }

    fn members(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let members = PyDict::new(py);
        for member in self.class.bind(py).try_iter()? {
            let member = member?;
            members.set_item(member.getattr(intern!(py, "value"))?, member)?;
        }
        Ok(members.unbind())
    }
}
