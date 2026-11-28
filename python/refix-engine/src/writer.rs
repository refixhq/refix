use pyo3::exceptions::PyTypeError;
use pyo3::types::{PyAnyMethods, PyBool, PyInt, PyString, PyStringMethods, PyTypeMethods};
use pyo3::{Bound, PyAny, PyErr, PyResult, import_exception, intern, pyclass, pymethods};
use refix_message::{FieldWriter as CoreFieldWriter, Tag, WriteError as CoreWriteError};

use crate::raw::{DECIMAL_TYPE, RawMessage};

import_exception!(refix.errors, WriteError);

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

    /// Return the fields written as a message, and start again with none.
    fn finish(&mut self) -> RawMessage {
        RawMessage::new(std::mem::take(&mut self.0).finish())
    }
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
