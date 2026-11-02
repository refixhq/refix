use pyo3::{pyclass, pymethods};
use refix_message::Tag;

/// How a counterparty's FIX differs from the standard.
#[pyclass(frozen, module = "refix._core")]
pub(crate) struct Dialect {
    extra_length_tags: Vec<Tag>,
}

impl Dialect {
    pub(crate) fn dialect(&self) -> refix_message::Dialect<'_> {
        refix_message::Dialect::new().with_extra_length_tags(&self.extra_length_tags)
    }
}

#[pymethods]
impl Dialect {
    #[new]
    #[pyo3(signature = (*, extra_length_tags = Vec::new()))]
    fn new(extra_length_tags: Vec<u32>) -> Self {
        Self {
            extra_length_tags: extra_length_tags.into_iter().map(Tag).collect(),
        }
    }
}
