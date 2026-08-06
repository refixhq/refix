//! Generates typed message wrappers from a [`refix_dictionary::Dictionary`].
//! Emits Rust source as plain strings; generated output is checked in and
//! kept fresh by tests that regenerate and compare.

mod case_converter;
mod emitter;

pub use case_converter::snake_case;
pub use emitter::generate;
