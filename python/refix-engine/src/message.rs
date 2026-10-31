use pyo3::sync::PyOnceLock;
use pyo3::types::{
    PyAnyMethods, PyBytes, PyDate, PyDateTime, PyString, PyTime, PyTuple, PyType, PyTzInfo,
};
use pyo3::{Bound, Py, PyAny, PyErr, PyResult, Python, import_exception, pyclass, pymethods};
use refix_message::{
    Group, InvalidValue, RawMessage as CoreRawMessage, Scope as CoreScope, Slot, Tag,
};

use crate::group::{GroupTable, KnownTags};

import_exception!(refix.errors, InvalidValueError);

/// Python's `decimal.Decimal`, imported on first use.
static DECIMAL_TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();

/// `refix.values.MonthYear`, imported on first use.
static MONTH_YEAR_TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();

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

    fn get_bool(&self, tag: u32) -> PyResult<Option<bool>> {
        get_bool(&self.0.scope(), tag)
    }

    fn get_decimal<'py>(&self, py: Python<'py>, tag: u32) -> PyResult<Option<Bound<'py, PyAny>>> {
        get_decimal(py, &self.0.scope(), tag)
    }

    fn get_utc_timestamp<'py>(
        &self,
        py: Python<'py>,
        tag: u32,
    ) -> PyResult<Option<Bound<'py, PyDateTime>>> {
        get_utc_timestamp(py, &self.0.scope(), tag)
    }

    fn get_utc_timestamp_ns(&self, tag: u32) -> PyResult<Option<i64>> {
        get_utc_timestamp_ns(&self.0.scope(), tag)
    }

    fn get_date<'py>(&self, py: Python<'py>, tag: u32) -> PyResult<Option<Bound<'py, PyDate>>> {
        get_date(py, &self.0.scope(), tag)
    }

    fn get_time<'py>(&self, py: Python<'py>, tag: u32) -> PyResult<Option<Bound<'py, PyTime>>> {
        get_time(py, &self.0.scope(), tag)
    }

    fn get_time_ns(&self, tag: u32) -> PyResult<Option<i64>> {
        get_time_ns(&self.0.scope(), tag)
    }

    fn get_month_year<'py>(
        &self,
        py: Python<'py>,
        tag: u32,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        get_month_year(py, &self.0.scope(), tag)
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

    fn get_bool(&self, tag: u32) -> PyResult<Option<bool>> {
        get_bool(&self.scope(), tag)
    }

    fn get_decimal<'py>(&self, py: Python<'py>, tag: u32) -> PyResult<Option<Bound<'py, PyAny>>> {
        get_decimal(py, &self.scope(), tag)
    }

    fn get_utc_timestamp<'py>(
        &self,
        py: Python<'py>,
        tag: u32,
    ) -> PyResult<Option<Bound<'py, PyDateTime>>> {
        get_utc_timestamp(py, &self.scope(), tag)
    }

    fn get_utc_timestamp_ns(&self, tag: u32) -> PyResult<Option<i64>> {
        get_utc_timestamp_ns(&self.scope(), tag)
    }

    fn get_date<'py>(&self, py: Python<'py>, tag: u32) -> PyResult<Option<Bound<'py, PyDate>>> {
        get_date(py, &self.scope(), tag)
    }

    fn get_time<'py>(&self, py: Python<'py>, tag: u32) -> PyResult<Option<Bound<'py, PyTime>>> {
        get_time(py, &self.scope(), tag)
    }

    fn get_time_ns(&self, tag: u32) -> PyResult<Option<i64>> {
        get_time_ns(&self.scope(), tag)
    }

    fn get_month_year<'py>(
        &self,
        py: Python<'py>,
        tag: u32,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        get_month_year(py, &self.scope(), tag)
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

fn get_bool(scope: &CoreScope<'_>, tag: u32) -> PyResult<Option<bool>> {
    scope.get_bool(Tag(tag)).map_err(to_py_err)
}

fn get_decimal<'py>(
    py: Python<'py>,
    scope: &CoreScope<'_>,
    tag: u32,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    let Some(value) = scope.get_decimal(Tag(tag)).map_err(to_py_err)? else {
        return Ok(None);
    };

    let decimal = DECIMAL_TYPE.import(py, "decimal", "Decimal")?;
    decimal.call1((value.to_string(),)).map(Some)
}

fn get_utc_timestamp<'py>(
    py: Python<'py>,
    scope: &CoreScope<'_>,
    tag: u32,
) -> PyResult<Option<Bound<'py, PyDateTime>>> {
    let Some(value) = scope.get_utc_timestamp(Tag(tag)).map_err(to_py_err)? else {
        return Ok(None);
    };

    let date = value.date();
    let (hour, minute, second, microsecond) = clock(value.time().nanos_since_midnight());
    let utc = PyTzInfo::utc(py)?;
    PyDateTime::new(
        py,
        i32::from(date.year()),
        date.month(),
        date.day(),
        hour,
        minute,
        second,
        microsecond,
        Some(&utc),
    )
    .map(Some)
}

fn get_utc_timestamp_ns(scope: &CoreScope<'_>, tag: u32) -> PyResult<Option<i64>> {
    let value = scope.get_utc_timestamp(Tag(tag)).map_err(to_py_err)?;
    Ok(value.map(|value| value.unix_nanos()))
}

fn get_date<'py>(
    py: Python<'py>,
    scope: &CoreScope<'_>,
    tag: u32,
) -> PyResult<Option<Bound<'py, PyDate>>> {
    let Some(value) = scope.get_date(Tag(tag)).map_err(to_py_err)? else {
        return Ok(None);
    };

    // Python's dates start at year 1.
    if value.year() == 0 {
        return Err(to_py_err(InvalidValue { tag: Tag(tag) }));
    }
    PyDate::new(py, i32::from(value.year()), value.month(), value.day()).map(Some)
}

fn get_time<'py>(
    py: Python<'py>,
    scope: &CoreScope<'_>,
    tag: u32,
) -> PyResult<Option<Bound<'py, PyTime>>> {
    let Some(value) = scope.get_time(Tag(tag)).map_err(to_py_err)? else {
        return Ok(None);
    };

    // Python's times stop at 23:59:59.999999, so a leap second clamps to it.
    let nanos = value.nanos_since_midnight().min(86_399_999_999_999);
    let (hour, minute, second, microsecond) = clock(nanos);
    PyTime::new(py, hour, minute, second, microsecond, None).map(Some)
}

fn get_time_ns(scope: &CoreScope<'_>, tag: u32) -> PyResult<Option<i64>> {
    let value = scope.get_time(Tag(tag)).map_err(to_py_err)?;
    Ok(value.map(|value| value.nanos_since_midnight()))
}

/// The hour, minute, second and microsecond of a time within a day, truncating nanoseconds.
fn clock(nanos_since_midnight: i64) -> (u8, u8, u8, u32) {
    let seconds = nanos_since_midnight / 1_000_000_000;
    let microsecond = nanos_since_midnight % 1_000_000_000 / 1_000;
    (
        (seconds / 3_600) as u8,
        (seconds / 60 % 60) as u8,
        (seconds % 60) as u8,
        microsecond as u32,
    )
}

fn get_month_year<'py>(
    py: Python<'py>,
    scope: &CoreScope<'_>,
    tag: u32,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    let Some(value) = scope.get_month_year(Tag(tag)).map_err(to_py_err)? else {
        return Ok(None);
    };

    let month_year = MONTH_YEAR_TYPE.import(py, "refix.values", "MonthYear")?;
    month_year
        .call1((value.year(), value.month(), value.day(), value.week()))
        .map(Some)
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
