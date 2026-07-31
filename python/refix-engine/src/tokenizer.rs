use bytes::Bytes;
use pyo3::exceptions::PyValueError;
use pyo3::{PyResult, pyclass, pymethods};
use refix_message::Tokenizer as CoreTokenizer;

#[pyclass(frozen, module = "refix._core")]
struct Tokenizer(CoreTokenizer);

#[pymethods]
impl Tokenizer {
    #[new]
    fn new() -> Self {
        Self(CoreTokenizer)
    }

    fn tokenize(&self, data: &[u8]) -> PyResult<crate::message::RawMessage> {
        self.0
            .tokenize(Bytes::copy_from_slice(data))
            .map(crate::message::RawMessage::new)
            .map_err(|err| PyValueError::new_err(format!("{err:?}")))
    }
}
