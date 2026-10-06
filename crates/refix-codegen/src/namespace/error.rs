use std::fmt;

use crate::Owner;

/// A name that two parts of the generated code would both use.
#[derive(Debug, Eq, PartialEq)]
pub struct NameClash {
    pub name: String,
    pub first: Owner,
    pub second: Owner,
}

impl fmt::Display for NameClash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` is taken by both {} and {}",
            self.name, self.first, self.second
        )
    }
}

impl std::error::Error for NameClash {}
