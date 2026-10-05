use pyo3::types::{PyBytes, PyString, PyTuple};
use pyo3::{Bound, Py, PyErr, PyResult, Python, import_exception, pyclass, pymethods};
use refix_message::{
    Group, InvalidValue, RawMessage as CoreRawMessage, Scope as CoreScope, Slot, Tag,
};

use crate::group::{GroupTable, KnownTags};

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
        get(py, &self.0.scope(), tag)
    }

    fn get_str<'py>(&self, py: Python<'py>, tag: u32) -> PyResult<Option<Bound<'py, PyString>>> {
        get_str(py, &self.0.scope(), tag)
    }

    fn get_int(&self, tag: u32) -> PyResult<Option<i64>> {
        get_int(&self.0.scope(), tag)
    }

    fn get_multiple_values<'py>(
        &self,
        py: Python<'py>,
        tag: u32,
    ) -> PyResult<Option<Bound<'py, PyTuple>>> {
        get_multiple_values(py, &self.0.scope(), tag)
    }

    fn get_group<'py>(
        slf: &Bound<'py, Self>,
        table: &Bound<'py, GroupTable>,
        known: &Bound<'py, KnownTags>,
    ) -> PyResult<Option<Bound<'py, PyTuple>>> {
        let message = slf.clone().unbind();
        get_group(slf.py(), &message, &slf.get().0.scope(), table, known)
    }

    fn entries<'py>(&self, py: Python<'py>) -> Vec<(u32, Bound<'py, PyBytes>)> {
        self.0
            .entries()
            .map(|(tag, value)| (tag.0, PyBytes::new(py, value)))
            .collect()
    }
}

/// One group instance, or any other bounded part of a message.
///
/// Holds its message and its bounds, and rebuilds the Rust scope per call.
#[pyclass(frozen, module = "refix._core")]
pub(crate) struct Scope {
    message: Py<RawMessage>,
    start: Slot,
    end: Slot,
}

impl Scope {
    fn scope(&self) -> CoreScope<'_> {
        self.message
            .get()
            .0
            .scope_between(self.start, self.end)
            .expect("a scope's bounds come from its own message")
    }
}

#[pymethods]
impl Scope {
    fn get<'py>(&self, py: Python<'py>, tag: u32) -> Option<Bound<'py, PyBytes>> {
        get(py, &self.scope(), tag)
    }

    fn get_str<'py>(&self, py: Python<'py>, tag: u32) -> PyResult<Option<Bound<'py, PyString>>> {
        get_str(py, &self.scope(), tag)
    }

    fn get_int(&self, tag: u32) -> PyResult<Option<i64>> {
        get_int(&self.scope(), tag)
    }

    fn get_multiple_values<'py>(
        &self,
        py: Python<'py>,
        tag: u32,
    ) -> PyResult<Option<Bound<'py, PyTuple>>> {
        get_multiple_values(py, &self.scope(), tag)
    }

    fn get_group<'py>(
        &self,
        py: Python<'py>,
        table: &Bound<'py, GroupTable>,
        known: &Bound<'py, KnownTags>,
    ) -> PyResult<Option<Bound<'py, PyTuple>>> {
        get_group(py, &self.message, &self.scope(), table, known)
    }
}

fn get<'py>(py: Python<'py>, scope: &CoreScope<'_>, tag: u32) -> Option<Bound<'py, PyBytes>> {
    scope.get(Tag(tag)).map(|value| PyBytes::new(py, value))
}

fn get_str<'py>(
    py: Python<'py>,
    scope: &CoreScope<'_>,
    tag: u32,
) -> PyResult<Option<Bound<'py, PyString>>> {
    let value = scope.get_str(Tag(tag)).map_err(to_py_err)?;
    Ok(value.map(|value| PyString::new(py, value)))
}

fn get_int(scope: &CoreScope<'_>, tag: u32) -> PyResult<Option<i64>> {
    scope.get_int(Tag(tag)).map_err(to_py_err)
}

fn get_multiple_values<'py>(
    py: Python<'py>,
    scope: &CoreScope<'_>,
    tag: u32,
) -> PyResult<Option<Bound<'py, PyTuple>>> {
    let Some(values) = scope
        .get_multiple_values::<&str>(Tag(tag))
        .map_err(to_py_err)?
    else {
        return Ok(None);
    };
    let values: Vec<&str> = values.iter().collect();
    PyTuple::new(py, values).map(Some)
}

/// The group `table` describes as one [`Scope`] per instance, each holding
/// `message`.
fn get_group<'py>(
    py: Python<'py>,
    message: &Py<RawMessage>,
    scope: &CoreScope<'_>,
    table: &Bound<'py, GroupTable>,
    known: &Bound<'py, KnownTags>,
) -> PyResult<Option<Bound<'py, PyTuple>>> {
    let group: Option<Group<'_>> = scope
        .get_group(table.get().table(), &known.get().known())
        .map_err(to_py_err)?;
    let Some(group) = group else {
        return Ok(None);
    };
    let instances = group
        .iter()
        .map(|instance| {
            Py::new(
                py,
                Scope {
                    message: message.clone_ref(py),
                    start: instance.start(),
                    end: instance.end(),
                },
            )
        })
        .collect::<PyResult<Vec<_>>>()?;
    PyTuple::new(py, instances).map(Some)
}

fn to_py_err(error: InvalidValue) -> PyErr {
    InvalidValueError::new_err((error.tag.0,))
}
