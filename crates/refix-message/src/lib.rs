#![doc = include_str!("../README.md")]

mod dialect;
pub mod framing;
pub mod group;
mod interop;
mod length_tags;
pub mod log;
mod message;
pub mod multiple_values;
pub mod stream;
mod tag;
#[cfg(test)]
mod test_utils;
mod tokenizer;
mod value;

pub use dialect::Dialect;
pub use group::{
    Group, GroupLayout, GroupTable, GroupTableError, Instances, KnownTags, OwnedGroupTable,
};
pub use log::LogReader;
pub use message::{RawMessage, Scope, Slot};
pub use multiple_values::MultipleValues;
pub use stream::MessageStream;
pub use tag::Tag;
pub use tokenizer::{TokenizeError, Tokenizer};
pub use value::{
    Date, Decimal, InvalidValue, MonthYear, OutOfRange, Time, TimePrecision, UtcTimestamp,
};
