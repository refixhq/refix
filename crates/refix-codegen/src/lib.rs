//! Generates typed message wrappers from a [`refix_dictionary::Dictionary`].
//! Emits Rust source as plain strings; generated output is checked in and
//! kept fresh by tests that regenerate and compare.

mod case_converter;
mod emitter;

pub use case_converter::{pascal_case, snake_case};
pub use emitter::{Error, Generated, Warning, generate};
