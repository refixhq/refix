use pyo3::conversion::FromPyObjectOwned;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::sync::PyOnceLock;
use pyo3::types::{
    PyAnyMethods, PyBool, PyDate, PyDateTime, PyInt, PyString, PyStringMethods, PyTime,
    PyTypeMethods,
};
use pyo3::{Bound, Py, PyAny, PyErr, PyResult, import_exception, intern, pyclass, pymethods};
use refix_message::{
    Date, FieldWriter as CoreFieldWriter, MonthYear, Tag, Time, TimePrecision, UtcTimestamp,
    WriteError as CoreWriteError,
};

use crate::raw::{DECIMAL_TYPE, MONTH_YEAR_TYPE, RawMessage};

import_exception!(refix.errors, WriteError);

/// 1970-01-01 00:00:00 UTC as a `datetime`, made on first use.
static EPOCH: PyOnceLock<Py<PyAny>> = PyOnceLock::new();

/// Writes fields and gives them back as a `RawMessage`.
///
/// The fields are written in the order of the calls. A value that cannot be
/// written raises `WriteError`, and nothing is written for it.
#[pyclass(module = "refix._core")]
#[derive(Default)]
pub(crate) struct FieldWriter(CoreFieldWriter);

#[pymethods]
impl FieldWriter {
    #[new]
    fn new() -> Self {
        Self::default()
    }

    /// Write a field with `value` as its bytes.
    ///
    /// An empty value and a value that holds the separator raise `WriteError`.
    fn write(&mut self, tag: u32, value: &[u8]) -> PyResult<()> {
        self.0.write(Tag(tag), value).map_err(to_py_err)
    }

    /// Write a field with `value` as its text.
    ///
    /// An empty value and a value that holds the separator raise `WriteError`.
    fn write_str(&mut self, tag: u32, value: &str) -> PyResult<()> {
        self.0.write_str(Tag(tag), value).map_err(to_py_err)
    }

    /// Write a field with `value` as an integer.
    ///
    /// A `bool` raises `TypeError`, as `write_bool` is the method for it.
    fn write_int(&mut self, tag: u32, value: &Bound<'_, PyAny>) -> PyResult<()> {
        if value.is_instance_of::<PyBool>() {
            return Err(wrong_type("write_int", "an int", value));
        }
        self.0.write_int(Tag(tag), value.extract()?);
        Ok(())
    }

    /// Write a field with `Y` for `True` or `N` for `False`.
    fn write_bool(&mut self, tag: u32, value: bool) {
        self.0.write_bool(Tag(tag), value);
    }

    /// Write a field with `value` as a decimal, keeping its scale.
    ///
    /// Takes a `Decimal` or an `int`. NaN and infinity raise `WriteError`.
    fn write_decimal(&mut self, tag: u32, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let py = value.py();
        let is_int = value.is_instance_of::<PyInt>() && !value.is_instance_of::<PyBool>();
        if !is_int && !value.is_instance(DECIMAL_TYPE.import(py, "decimal", "Decimal")?)? {
            return Err(wrong_type("write_decimal", "a Decimal or an int", value));
        }
        // Neither form has an exponent.
        let form = if is_int {
            intern!(py, "d")
        } else {
            intern!(py, "f")
        };
        let text = value.call_method1(intern!(py, "__format__"), (form,))?;
        let text = text.cast::<PyString>()?.to_str()?;
        if !text
            .trim_start_matches('-')
            .starts_with(|first: char| first.is_ascii_digit())
        {
            let message = format!("{text} cannot be written to field {tag}");
            return Err(WriteError::new_err((tag, message)));
        }
        self.0.write_str(Tag(tag), text).map_err(to_py_err)
    }

    /// Write a field with `value` as a date, such as `20261103`.
    ///
    /// A `datetime` raises `TypeError`.
    fn write_date(&mut self, tag: u32, value: &Bound<'_, PyAny>) -> PyResult<()> {
        if !value.is_instance_of::<PyDate>() || value.is_instance_of::<PyDateTime>() {
            return Err(wrong_type("write_date", "a date", value));
        }
        let py = value.py();
        let date = part(value, intern!(py, "year"))
            .zip(part(value, intern!(py, "month")))
            .zip(part(value, intern!(py, "day")))
            .and_then(|((year, month), day)| Date::new(year, month, day))
            .ok_or_else(|| unwritable(tag, value))?;
        self.0.write_date(Tag(tag), date);
        Ok(())
    }

    /// Write a field with `value` as a time of day, such as `09:30:36.250`.
    ///
    /// The fraction has `precision` digits, from 0 to 9. Digits past it are cut,
    /// not rounded. A `time` with a time zone raises `WriteError`.
    #[pyo3(signature = (tag, value, *, precision = 3))]
    fn write_time(&mut self, tag: u32, value: &Bound<'_, PyAny>, precision: u8) -> PyResult<()> {
        if !value.is_instance_of::<PyTime>() {
            return Err(wrong_type("write_time", "a time", value));
        }
        let precision = time_precision(precision)?;
        let py = value.py();
        if !value.getattr(intern!(py, "tzinfo"))?.is_none() {
            return Err(unwritable(tag, value));
        }
        let seconds = part::<i64>(value, intern!(py, "hour"))
            .zip(part::<i64>(value, intern!(py, "minute")))
            .zip(part::<i64>(value, intern!(py, "second")))
            .map(|((hour, minute), second)| (hour * 60 + minute) * 60 + second);
        let nanos = seconds
            .zip(part::<i64>(value, intern!(py, "microsecond")))
            .map(|(seconds, microsecond)| seconds * NANOS_PER_SECOND + microsecond * 1_000);
        let time = nanos
            .and_then(|nanos| Time::new(nanos, precision))
            .ok_or_else(|| unwritable(tag, value))?;
        self.0.write_time(Tag(tag), time);
        Ok(())
    }

    /// Write a field with `value`, nanoseconds since midnight, as a time of day.
    ///
    /// The fraction has `precision` digits, from 0 to 9. Digits past it are cut,
    /// not rounded. A value outside a day raises `WriteError`.
    #[pyo3(signature = (tag, value, *, precision = 9))]
    fn write_time_ns(&mut self, tag: u32, value: i64, precision: u8) -> PyResult<()> {
        let time = Time::new(value, time_precision(precision)?).ok_or_else(|| {
            let message = format!("{value} ns is not a time of day, for field {tag}");
            WriteError::new_err((tag, message))
        })?;
        self.0.write_time(Tag(tag), time);
        Ok(())
    }

    /// Write a field with `value` as a UTC timestamp, such as `20261103-09:30:36.250`.
    ///
    /// Takes a `datetime` with a time zone and converts it to UTC. The fraction
    /// has `precision` digits, from 0 to 9. Digits past it are cut, not rounded.
    /// Raises `WriteError` without a time zone, and outside what nanoseconds
    /// since 1970 can hold, about 1678 to 2262.
    #[pyo3(signature = (tag, value, *, precision = 3))]
    fn write_utc_timestamp(
        &mut self,
        tag: u32,
        value: &Bound<'_, PyAny>,
        precision: u8,
    ) -> PyResult<()> {
        if !value.is_instance_of::<PyDateTime>() {
            return Err(wrong_type("write_utc_timestamp", "a datetime", value));
        }
        let precision = time_precision(precision)?;
        let nanos = unix_nanos(value)?.ok_or_else(|| unwritable(tag, value))?;
        self.0
            .write_utc_timestamp(Tag(tag), UtcTimestamp::new(nanos, precision));
        Ok(())
    }

    /// Write a field with `value`, nanoseconds since 1970 in UTC, as a UTC timestamp.
    ///
    /// The fraction has `precision` digits, from 0 to 9. Digits past it are cut,
    /// not rounded.
    #[pyo3(signature = (tag, value, *, precision = 9))]
    fn write_utc_timestamp_ns(&mut self, tag: u32, value: i64, precision: u8) -> PyResult<()> {
        let stamp = UtcTimestamp::new(value, time_precision(precision)?);
        self.0.write_utc_timestamp(Tag(tag), stamp);
        Ok(())
    }

    /// Write a field with `value` as a month and year, such as `202612`.
    ///
    /// Takes a `refix.MonthYear`. One that names no real month, day or week
    /// raises `WriteError`.
    fn write_month_year(&mut self, tag: u32, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let py = value.py();
        let class = MONTH_YEAR_TYPE.import(py, "refix.values", "MonthYear")?;
        if !value.is_instance(class)? {
            return Err(wrong_type("write_month_year", "a MonthYear", value));
        }
        let day = part::<Option<u8>>(value, intern!(py, "day"));
        let week = part::<Option<u8>>(value, intern!(py, "week"));
        let month_year = part(value, intern!(py, "year"))
            .zip(part(value, intern!(py, "month")))
            .zip(day.zip(week))
            .and_then(|((year, month), day_and_week)| match day_and_week {
                (None, None) => MonthYear::new(year, month),
                (Some(day), None) => MonthYear::with_day(year, month, day),
                (None, Some(week)) => MonthYear::with_week(year, month, week),
                (Some(_), Some(_)) => None,
            })
            .ok_or_else(|| unwritable(tag, value))?;
        self.0.write_month_year(Tag(tag), month_year);
        Ok(())
    }

    /// Return the fields written as a message, and start again with none.
    fn finish(&mut self) -> RawMessage {
        RawMessage::new(std::mem::take(&mut self.0).finish())
    }
}

const NANOS_PER_SECOND: i64 = 1_000_000_000;

/// The attribute `name` of `value`, when it is there and fits the type.
fn part<'py, T: FromPyObjectOwned<'py>>(
    value: &Bound<'py, PyAny>,
    name: &Bound<'py, PyString>,
) -> Option<T> {
    value.getattr(name).ok()?.extract().ok()
}

fn time_precision(digits: u8) -> PyResult<TimePrecision> {
    TimePrecision::from_fraction_digits(digits)
        .ok_or_else(|| PyValueError::new_err("precision must be from 0 to 9 digits"))
}

/// Nanoseconds from 1970 to `value`, a `datetime`.
///
/// `None` without a time zone or when the count does not fit.
fn unix_nanos(value: &Bound<'_, PyAny>) -> PyResult<Option<i64>> {
    let py = value.py();
    let epoch = EPOCH.get_or_try_init(py, || -> PyResult<Py<PyAny>> {
        let datetime = py.import("datetime")?;
        let utc = datetime.getattr("timezone")?.getattr("utc")?;
        let epoch = datetime
            .getattr("datetime")?
            .call1((1970, 1, 1, 0, 0, 0, 0, utc))?;
        Ok(epoch.unbind())
    })?;
    let since = match value.sub(epoch.bind(py)) {
        Ok(since) => since,
        // Python refuses to subtract when `value` has no time zone.
        Err(error) if error.is_instance_of::<PyTypeError>(py) => return Ok(None),
        Err(error) => return Err(error),
    };
    let days: i64 = since.getattr(intern!(py, "days"))?.extract()?;
    let seconds: i64 = since.getattr(intern!(py, "seconds"))?.extract()?;
    let microseconds: i64 = since.getattr(intern!(py, "microseconds"))?.extract()?;
    Ok((days * 86_400 + seconds)
        .checked_mul(NANOS_PER_SECOND)
        .and_then(|nanos| nanos.checked_add(microseconds * 1_000)))
}

/// The error for a `value` of the right type that has no FIX form.
fn unwritable(tag: u32, value: &Bound<'_, PyAny>) -> PyErr {
    let shown = value
        .repr()
        .map_or_else(|_| String::from("this value"), |repr| repr.to_string());
    WriteError::new_err((tag, format!("{shown} cannot be written to field {tag}")))
}

/// The error for a `value` that `method` does not take.
fn wrong_type(method: &str, expected: &str, value: &Bound<'_, PyAny>) -> PyErr {
    let given = value
        .get_type()
        .name()
        .map_or_else(|_| String::from("this value"), |name| name.to_string());
    PyTypeError::new_err(format!("{method} takes {expected}, not {given}"))
}

fn to_py_err(error: CoreWriteError) -> PyErr {
    let (CoreWriteError::EmptyValue { tag }
    | CoreWriteError::SeparatorInValue { tag }
    | CoreWriteError::ReservedTag { tag }) = error;
    WriteError::new_err((tag.0, error.to_string()))
}
