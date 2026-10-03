use refix_dictionary::MemberContext;

/// A construct the generator recognised but does not generate code for yet.
#[derive(Debug, Eq, PartialEq)]
pub enum Warning {
    UnsupportedGroup { context: MemberContext },
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Warning::UnsupportedGroup { context } => {
                write!(f, "{context} is not generated yet")
            }
        }
    }
}
