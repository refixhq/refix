/// A FIX tag number, e.g. `Tag(35)` for MsgType.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Tag(pub u32);

impl Tag {
    /// Tag recorded for a run of bytes that could not be tokenised into a field.
    pub const MALFORMED: Tag = Tag(0);
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_as_the_bare_number() {
        assert_eq!(Tag(35).to_string(), "35");
    }
}
