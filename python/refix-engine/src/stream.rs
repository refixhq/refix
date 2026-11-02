use crate::dialect::Dialect;
use crate::tokenizer::reason_str;
use bytes::Bytes;
use pyo3::types::{PyAnyMethods, PyBytes, PyModule};
use pyo3::{Bound, Py, PyAny, PyResult, Python, pyclass, pymethods};
use refix_message::framing::GarbledReason;
use refix_message::stream::Outcome;
use refix_message::{Dialect as CoreDialect, MessageStream as CoreMessageStream};

#[pyclass(module = "refix._core")]
pub(crate) struct MessageStream(CoreMessageStream);

#[pymethods]
impl MessageStream {
    #[new]
    #[pyo3(signature = (dialect = None))]
    fn new(dialect: Option<Bound<'_, Dialect>>) -> Self {
        let dialect = dialect
            .as_ref()
            .map_or_else(CoreDialect::new, |dialect| dialect.get().dialect());
        Self(CoreMessageStream::new(dialect))
    }

    fn feed(&mut self, data: &[u8]) {
        self.0.feed(data);
    }

    fn next_message(&mut self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        match self.0.next_message() {
            Outcome::Message(message) => {
                let message = crate::message::RawMessage::new(message);
                Ok(Some(Py::new(py, message)?.into_any()))
            }
            Outcome::Garbled { reason, bytes } => {
                Ok(Some(Py::new(py, Garble { reason, bytes })?.into_any()))
            }
            Outcome::Incomplete => Ok(None),
        }
    }
}

/// Bytes between messages that cannot begin one.
#[pyclass(frozen, module = "refix._core")]
pub(crate) struct Garble {
    reason: GarbledReason,
    bytes: Bytes,
}

#[pymethods]
impl Garble {
    #[getter]
    fn bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.bytes)
    }

    #[getter]
    fn reason<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let reasons = PyModule::import(py, "refix.errors")?.getattr("GarbledReason")?;
        reasons.call1((reason_str(self.reason),))
    }
}
