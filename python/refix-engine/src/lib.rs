mod dialect;
mod fields;
mod group;
mod log;
mod raw;
mod stream;
mod tokenizer;

use pyo3::prelude::*;

/// Return the package version.
#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_class::<dialect::Dialect>()?;
    m.add_class::<raw::RawMessage>()?;
    m.add_class::<raw::Scope>()?;
    m.add_class::<group::GroupTable>()?;
    m.add_class::<group::KnownTags>()?;
    m.add_class::<fields::Message>()?;
    m.add_class::<fields::Component>()?;
    m.add_class::<fields::GroupInstance>()?;
    m.add_class::<fields::Str>()?;
    m.add_class::<fields::Int>()?;
    m.add_class::<fields::Decimal>()?;
    m.add_class::<log::LogReader>()?;
    m.add_class::<stream::Garble>()?;
    m.add_class::<stream::MessageStream>()?;
    m.add_class::<tokenizer::Tokenizer>()?;
    m.add("MALFORMED_TAG", refix_message::Tag::MALFORMED.0)?;
    Ok(())
}
