use crate::Tag;

/// How a counterparty's FIX differs from the standard.
#[derive(Clone, Copy, Debug, Default)]
pub struct Dialect<'a> {
    pub(crate) extra_length_tags: &'a [Tag],
}

impl<'a> Dialect<'a> {
    /// Standard FIX.
    pub const fn new() -> Self {
        Self {
            extra_length_tags: &[],
        }
    }

    /// Length tags beyond the standard set, each giving the byte count of the
    /// data field after it.
    pub const fn with_extra_length_tags(self, tags: &'a [Tag]) -> Self {
        Self {
            extra_length_tags: tags,
        }
    }
}
