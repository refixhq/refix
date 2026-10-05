mod group;
mod message;
mod stream;
mod tokenizer;

use pyo3::prelude::*;

#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_class::<group::GroupTable>()?;
    m.add_class::<group::KnownTags>()?;
    m.add_class::<stream::Garble>()?;
    m.add_class::<stream::MessageStream>()?;
    m.add_class::<message::RawMessage>()?;
    m.add_class::<message::Scope>()?;
    m.add_class::<tokenizer::Tokenizer>()?;
    m.add("MALFORMED_TAG", refix_message::Tag::MALFORMED.0)?;
    Ok(())
}
