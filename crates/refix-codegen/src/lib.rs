#![doc = include_str!("../README.md")]

mod case_converter;
mod envelope;
mod generated;
mod groups;
mod literal;
mod namespace;
mod naming;
pub mod python;
pub mod rust;
#[cfg(test)]
mod test_utils;
mod warning;

pub use case_converter::{pascal_case, snake_case};
pub use generated::Generated;
pub use namespace::{NameClash, Owner};
pub use warning::Warning;
