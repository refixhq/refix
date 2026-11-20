use pyo3::sync::PyOnceLock;
use pyo3::types::{
    PyAnyMethods, PyBytes, PyDate, PyDateTime, PyString, PyTime, PyTuple, PyType, PyTzInfo,
};
use pyo3::{Bound, Py, PyAny, PyErr, PyResult, Python, import_exception, pyclass, pymethods};
use refix_message::{
    Group, InvalidValue, OwnedGroupTable, RawMessage as CoreRawMessage, Scope as CoreScope, Slot,
    Tag,
};

use crate::group::{GroupTable, KnownTags};

import_exception!(refix.errors, InvalidValueError);

/// Python's `decimal.Decimal`, imported on first use.
static DECIMAL_TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();

/// `refix.values.MonthYear`, imported on first use.
static MONTH_YEAR_TYPE: PyOnceLock<Py<PyType>> = PyOnceLock::new();

/// The bytes of a FIX message and an index of every field in wire order.
///
/// A typed getter raises `InvalidValueError` for a value it cannot read.
#[pyclass(frozen, module = "refix._core")]
pub(crate) struct RawMessage(CoreRawMessage);

impl RawMessage {
    pub(crate) fn new(inner: CoreRawMessage) -> Self {
        Self(inner)
    }

    pub(crate) fn scope(&self) -> CoreScope<'_> {
        self.0.scope()
    }
}

#[pymethods]
impl RawMessage {
    /// The bytes the message was read from.
    #[getter]
    fn bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.0.bytes())
    }

    /// Return the value of the first `tag` field, or `None` when absent.
    fn get<'py>(&self, py: Python<'py>, tag: u32) -> Option<Bound<'py, PyBytes>> {
        get(py, &self.0.scope(), tag)
    }

    /// Return the first `tag` field as text, or `None` when absent.
    fn get_str<'py>(&self, py: Python<'py>, tag: u32) -> PyResult<Option<Bound<'py, PyString>>> {
        get_str(py, &self.0.scope(), tag)
    }

    /// Return the first `tag` field as an integer, or `None` when absent.
    fn get_int(&self, tag: u32) -> PyResult<Option<i64>> {
        get_int(&self.0.scope(), tag)
    }

    /// Return the first `tag` field as a boolean, or `None` when absent.
    fn get_bool(&self, tag: u32) -> PyResult<Option<bool>> {
        get_bool(&self.0.scope(), tag)
    }

    /// Return the first `tag` field as a `Decimal`, or `None` when absent.
    fn get_decimal<'py>(&self, py: Python<'py>, tag: u32) -> PyResult<Option<Bound<'py, PyAny>>> {
        get_decimal(py, &self.0.scope(), tag)
    }

    /// Return the first `tag` field as a UTC `datetime`, or `None` when absent.
    ///
    /// Truncates to the microsecond.
    fn get_utc_timestamp<'py>(
        &self,
        py: Python<'py>,
        tag: u32,
    ) -> PyResult<Option<Bound<'py, PyDateTime>>> {
        get_utc_timestamp(py, &self.0.scope(), tag)
    }

    /// Return the first `tag` field as nanoseconds since the Unix epoch, or `None` when absent.
    fn get_utc_timestamp_ns(&self, tag: u32) -> PyResult<Option<i64>> {
        get_utc_timestamp_ns(&self.0.scope(), tag)
    }

    /// Return the first `tag` field as a `date`, or `None` when absent.
    fn get_date<'py>(&self, py: Python<'py>, tag: u32) -> PyResult<Option<Bound<'py, PyDate>>> {
        get_date(py, &self.0.scope(), tag)
    }

    /// Return the first `tag` field as a naive `time`, or `None` when absent.
    ///
    /// Truncates to the microsecond.
    fn get_time<'py>(&self, py: Python<'py>, tag: u32) -> PyResult<Option<Bound<'py, PyTime>>> {
        get_time(py, &self.0.scope(), tag)
    }

    /// Return the first `tag` field as nanoseconds since midnight, or `None` when absent.
    fn get_time_ns(&self, tag: u32) -> PyResult<Option<i64>> {
        get_time_ns(&self.0.scope(), tag)
    }

    /// Return the first `tag` field as a `MonthYear`, or `None` when absent.
    fn get_month_year<'py>(
        &self,
        py: Python<'py>,
        tag: u32,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        get_month_year(py, &self.0.scope(), tag)
    }

    /// Return the space-separated values of the first `tag` field, or `None` when absent.
    fn get_multiple_values<'py>(
        &self,
        py: Python<'py>,
        tag: u32,
    ) -> PyResult<Option<Bound<'py, PyTuple>>> {
        get_multiple_values(py, &self.0.scope(), tag)
    }

    /// Return the instances of the group `table` describes, or `None` when absent.
    ///
    /// `known` holds every tag that can appear where the group is read.
    fn get_group<'py>(
        slf: &Bound<'py, Self>,
        table: &Bound<'py, GroupTable>,
        known: &Bound<'py, KnownTags>,
    ) -> PyResult<Option<Bound<'py, PyTuple>>> {
        let message = slf.clone().unbind();
        get_group(slf.py(), &message, &slf.get().0.scope(), table, known)
    }

    /// Return every field as `(tag, value)` in wire order.
    ///
    /// A run of bytes that is not a field has the tag `MALFORMED_TAG`.
    fn entries<'py>(&self, py: Python<'py>) -> Vec<(u32, Bound<'py, PyBytes>)> {
        self.0
            .entries()
            .map(|(tag, value)| (tag.0, PyBytes::new(py, value)))
            .collect()
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }
}

/// One group instance, or any other bounded part of a message.
///
/// A typed getter raises `InvalidValueError` for a value it cannot read.
// Holds its message and its bounds, and rebuilds the Rust scope per call.
#[pyclass(frozen, module = "refix._core")]
pub(crate) struct Scope {
    message: Py<RawMessage>,
    start: Slot,
    end: Slot,
}

impl Scope {
    pub(crate) fn message(&self, py: Python<'_>) -> Py<RawMessage> {
        self.message.clone_ref(py)
    }

    pub(crate) fn scope(&self) -> CoreScope<'_> {
        self.message
            .get()
            .0
            .scope_between(self.start, self.end)
            .expect("a scope's bounds come from its own message")
    }
}

#[pymethods]
impl Scope {
    /// Return the value of the first `tag` field, or `None` when absent.
    fn get<'py>(&self, py: Python<'py>, tag: u32) -> Option<Bound<'py, PyBytes>> {
        get(py, &self.scope(), tag)
    }

    /// Return the first `tag` field as text, or `None` when absent.
    fn get_str<'py>(&self, py: Python<'py>, tag: u32) -> PyResult<Option<Bound<'py, PyString>>> {
        get_str(py, &self.scope(), tag)
    }

    /// Return the first `tag` field as an integer, or `None` when absent.
    fn get_int(&self, tag: u32) -> PyResult<Option<i64>> {
        get_int(&self.scope(), tag)
    }

    /// Return the first `tag` field as a boolean, or `None` when absent.
    fn get_bool(&self, tag: u32) -> PyResult<Option<bool>> {
        get_bool(&self.scope(), tag)
    }

    /// Return the first `tag` field as a `Decimal`, or `None` when absent.
    fn get_decimal<'py>(&self, py: Python<'py>, tag: u32) -> PyResult<Option<Bound<'py, PyAny>>> {
        get_decimal(py, &self.scope(), tag)
    }

    /// Return the first `tag` field as a UTC `datetime`, or `None` when absent.
    ///
    /// Truncates to the microsecond.
    fn get_utc_timestamp<'py>(
        &self,
        py: Python<'py>,
        tag: u32,
    ) -> PyResult<Option<Bound<'py, PyDateTime>>> {
        get_utc_timestamp(py, &self.scope(), tag)
    }

    /// Return the first `tag` field as nanoseconds since the Unix epoch, or `None` when absent.
    fn get_utc_timestamp_ns(&self, tag: u32) -> PyResult<Option<i64>> {
        get_utc_timestamp_ns(&self.scope(), tag)
    }

    /// Return the first `tag` field as a `date`, or `None` when absent.
    fn get_date<'py>(&self, py: Python<'py>, tag: u32) -> PyResult<Option<Bound<'py, PyDate>>> {
        get_date(py, &self.scope(), tag)
    }

    /// Return the first `tag` field as a naive `time`, or `None` when absent.
    ///
    /// Truncates to the microsecond.
    fn get_time<'py>(&self, py: Python<'py>, tag: u32) -> PyResult<Option<Bound<'py, PyTime>>> {
        get_time(py, &self.scope(), tag)
    }

    /// Return the first `tag` field as nanoseconds since midnight, or `None` when absent.
    fn get_time_ns(&self, tag: u32) -> PyResult<Option<i64>> {
        get_time_ns(&self.scope(), tag)
    }

    /// Return the first `tag` field as a `MonthYear`, or `None` when absent.
    fn get_month_year<'py>(
        &self,
        py: Python<'py>,
        tag: u32,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        get_month_year(py, &self.scope(), tag)
    }

    /// Return the space-separated values of the first `tag` field, or `None` when absent.
    fn get_multiple_values<'py>(
        &self,
        py: Python<'py>,
        tag: u32,
    ) -> PyResult<Option<Bound<'py, PyTuple>>> {
        get_multiple_values(py, &self.scope(), tag)
    }

    /// Return the instances of the group `table` describes, or `None` when absent.
    ///
    /// `known` holds every tag that can appear where the group is read.
    fn get_group<'py>(
        &self,
        py: Python<'py>,
        table: &Bound<'py, GroupTable>,
        known: &Bound<'py, KnownTags>,
    ) -> PyResult<Option<Bound<'py, PyTuple>>> {
        get_group(py, &self.message, &self.scope(), table, known)
    }

    fn __str__(&self) -> String {
        self.scope().to_string()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.scope())
    }
}

pub(crate) fn get<'py>(
    py: Python<'py>,
    scope: &CoreScope<'_>,
    tag: u32,
) -> Option<Bound<'py, PyBytes>> {
    scope.get(Tag(tag)).map(|value| PyBytes::new(py, value))
}

pub(crate) fn get_str<'py>(
    py: Python<'py>,
    scope: &CoreScope<'_>,
    tag: u32,
) -> PyResult<Option<Bound<'py, PyString>>> {
    let value = scope.get_str(Tag(tag)).map_err(to_py_err)?;
    Ok(value.map(|value| PyString::new(py, value)))
}

pub(crate) fn get_int(scope: &CoreScope<'_>, tag: u32) -> PyResult<Option<i64>> {
    scope.get_int(Tag(tag)).map_err(to_py_err)
}

pub(crate) fn get_bool(scope: &CoreScope<'_>, tag: u32) -> PyResult<Option<bool>> {
    scope.get_bool(Tag(tag)).map_err(to_py_err)
}

pub(crate) fn get_decimal<'py>(
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

pub(crate) fn get_utc_timestamp<'py>(
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

pub(crate) fn get_utc_timestamp_ns(scope: &CoreScope<'_>, tag: u32) -> PyResult<Option<i64>> {
    let value = scope.get_utc_timestamp(Tag(tag)).map_err(to_py_err)?;
    Ok(value.map(|value| value.unix_nanos()))
}

pub(crate) fn get_date<'py>(
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

pub(crate) fn get_time<'py>(
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

pub(crate) fn get_time_ns(scope: &CoreScope<'_>, tag: u32) -> PyResult<Option<i64>> {
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

pub(crate) fn get_month_year<'py>(
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

pub(crate) fn get_multiple_values<'py>(
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
    let scopes = group_scopes(
        py,
        message,
        scope,
        table.get().table(),
        &known.get().known(),
    )?;
    scopes.map(|scopes| PyTuple::new(py, scopes)).transpose()
}

/// One [`Scope`] per instance of the group `table` describes, each holding
/// `message`.
pub(super) fn group_scopes(
    py: Python<'_>,
    message: &Py<RawMessage>,
    scope: &CoreScope<'_>,
    table: &OwnedGroupTable,
    known: &refix_message::KnownTags<'_>,
) -> PyResult<Option<Vec<Py<Scope>>>> {
    let group: Option<Group<'_>> = scope.get_group(table, known).map_err(to_py_err)?;
    let Some(group) = group else {
        return Ok(None);
    };
    group
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
        .collect::<PyResult<Vec<_>>>()
        .map(Some)
}

fn to_py_err(error: InvalidValue) -> PyErr {
    InvalidValueError::new_err((error.tag.0,))
}
