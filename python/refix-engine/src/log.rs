use crate::dialect::Dialect;
use crate::message::RawMessage;
use pyo3::exceptions::PyRuntimeError;
use pyo3::types::PyBytes;
use pyo3::{Bound, PyResult, Python, pyclass, pymethods};
use refix_message::log::{LogDiagnostic, LogRecord};
use refix_message::{Dialect as CoreDialect, LogReader as CoreLogReader};

/// A record as handed to Python, a tuple of the prefix, the message and the diagnostics.
type Record<'py> = (Bound<'py, PyBytes>, RawMessage, Vec<&'static str>);

/// The reader behind `refix.read_log`.
#[pyclass(module = "refix._core")]
pub(crate) struct LogReader(Option<CoreLogReader>);

#[pymethods]
impl LogReader {
    #[new]
    #[pyo3(signature = (dialect = None, separator = None))]
    fn new(dialect: Option<Bound<'_, Dialect>>, separator: Option<u8>) -> Self {
        let dialect = dialect
            .as_ref()
            .map_or_else(CoreDialect::new, |dialect| dialect.get().dialect());
        Self(Some(CoreLogReader::new(dialect, separator)))
    }

    fn feed(&mut self, data: &[u8]) -> PyResult<()> {
        self.reader()?.feed(data);
        Ok(())
    }

    fn next_record<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Record<'py>>> {
        Ok(self
            .reader()?
            .next_record()
            .map(|record| convert(py, record)))
    }

    fn finish<'py>(&mut self, py: Python<'py>) -> PyResult<Vec<Record<'py>>> {
        let reader = self.0.take().ok_or_else(finished)?;
        Ok(reader.finish().map(|record| convert(py, record)).collect())
    }
}

impl LogReader {
    fn reader(&mut self) -> PyResult<&mut CoreLogReader> {
        self.0.as_mut().ok_or_else(finished)
    }
}

fn finished() -> pyo3::PyErr {
    PyRuntimeError::new_err("the reader is finished")
}

fn convert(py: Python<'_>, record: LogRecord) -> Record<'_> {
    (
        PyBytes::new(py, &record.prefix),
        RawMessage::new(record.message),
        record.diagnostics.iter().map(diagnostic_str).collect(),
    )
}

/// The `refix.LogDiagnostic` value of a diagnostic.
fn diagnostic_str(diagnostic: &LogDiagnostic) -> &'static str {
    match diagnostic {
        LogDiagnostic::MissingBodyLength => "missing_body_length",
        LogDiagnostic::MalformedBodyLength => "malformed_body_length",
        LogDiagnostic::BodyLengthMismatch => "body_length_mismatch",
        LogDiagnostic::MissingMsgType => "missing_msg_type",
        LogDiagnostic::MalformedChecksum => "malformed_checksum",
        LogDiagnostic::ChecksumMismatch => "checksum_mismatch",
        LogDiagnostic::Truncated => "truncated",
        LogDiagnostic::PrefixTruncated => "prefix_truncated",
    }
}
