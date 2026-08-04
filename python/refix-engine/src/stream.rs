use pyo3::{pyclass, pymethods};
use refix_message::{MessageStream as CoreMessageStream, Tokenizer as CoreTokenizer};

#[pyclass(module = "refix._core")]
pub(crate) struct MessageStream(CoreMessageStream);

#[pymethods]
impl MessageStream {
    #[new]
    #[pyo3(signature = (*, extra_length_tags = Vec::new()))]
    fn new(extra_length_tags: Vec<u32>) -> Self {
        let tokenizer = CoreTokenizer::with_extra_length_tags(extra_length_tags);
        Self(CoreMessageStream::new(tokenizer))
    }
}
