use std::fmt;

/// The message, component, group instance, header or trailer a member
/// appears in.
///
/// Diagnostics use this to name the place a problem was found.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum MemberContext {
    Message(String),
    Component(String),
    Header,
    Trailer,
    /// A group's instances, named after the group's count field.
    Group {
        name: String,
        parent: Box<MemberContext>,
    },
}

impl MemberContext {
    /// The context of a group's instances, nested in this context.
    pub fn group(&self, name: &str) -> MemberContext {
        MemberContext::Group {
            name: name.to_owned(),
            parent: Box::new(self.clone()),
        }
    }
}

impl fmt::Display for MemberContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(name) => write!(f, "message '{name}'"),
            Self::Component(name) => write!(f, "component '{name}'"),
            Self::Header => write!(f, "the header"),
            Self::Trailer => write!(f, "the trailer"),
            Self::Group { name, parent } => write!(f, "group '{name}' in {parent}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_group_names_the_header() {
        assert_eq!(
            MemberContext::Header.group("NoHops").to_string(),
            "group 'NoHops' in the header"
        );
        assert_eq!(MemberContext::Trailer.to_string(), "the trailer");
    }

    #[test]
    fn a_nested_group_context_names_its_whole_path() {
        let context = MemberContext::Group {
            name: "NoPartySubIDs".to_owned(),
            parent: Box::new(MemberContext::Group {
                name: "NoPartyIDs".to_owned(),
                parent: Box::new(MemberContext::Component("Parties".to_owned())),
            }),
        };

        assert_eq!(
            context.to_string(),
            "group 'NoPartySubIDs' in group 'NoPartyIDs' in component 'Parties'"
        );
    }
}
