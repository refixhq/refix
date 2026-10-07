use std::fmt;

/// A language the generator emits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Language {
    Rust,
    Python,
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::Rust => write!(f, "rust"),
            Language::Python => write!(f, "python"),
        }
    }
}
