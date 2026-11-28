use pyo3::{PyErr, PyResult, import_exception, pyclass, pymethods};
use refix_message::{FieldWriter as CoreFieldWriter, Tag, WriteError as CoreWriteError};

use crate::raw::RawMessage;

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

    /// Return the fields written as a message, and start again with none.
    fn finish(&mut self) -> RawMessage {
        RawMessage::new(std::mem::take(&mut self.0).finish())
    }
}

fn to_py_err(error: CoreWriteError) -> PyErr {
    let (CoreWriteError::EmptyValue { tag }
    | CoreWriteError::SeparatorInValue { tag }
    | CoreWriteError::ReservedTag { tag }) = error;
    WriteError::new_err((tag.0, error.to_string()))
}
