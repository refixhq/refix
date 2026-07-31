use pyo3::types::PyBytes;
use pyo3::{Bound, Python, pyclass, pymethods};
use refix_message::RawMessage as CoreRawMessage;

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
        self.0.get(tag).map(|value| PyBytes::new(py, value))
    }
}
