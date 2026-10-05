//! Generates typed message wrappers from a [`refix_dictionary::Dictionary`].
//! Emits Rust source as plain strings; generated output is checked in and
//! kept fresh by tests that regenerate and compare.

mod case_converter;
mod generated;
mod groups;
mod literal;
mod naming;
pub mod python;
pub mod rust;
#[cfg(test)]
mod test_utils;
mod warning;

pub use case_converter::{pascal_case, snake_case};
pub use generated::Generated;
pub use warning::Warning;
