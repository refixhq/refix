#![doc = include_str!("../README.md")]

mod case_converter;
mod components;
mod conversion;
mod dialect;
mod envelope;
mod flat;
mod groups;
mod literal;
mod namespace;
mod naming;
mod package;
pub mod python;
pub mod rust;
#[cfg(test)]
mod test_utils;
mod warning;

pub use case_converter::{pascal_case, snake_case};
pub use namespace::{NameClash, Owner};
pub use package::{Module, Package, WriteError};
pub use warning::Warning;
