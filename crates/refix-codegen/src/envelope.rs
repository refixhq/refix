use refix_dictionary::{Dictionary, MemberContext, dictionary};

use crate::flat;

/// The header or the trailer, each read through a view of its own.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Section {
    Header,
    Trailer,
}

impl Section {
    /// The view's type, named alike in every language.
    pub(crate) fn type_name(self) -> &'static str {
        match self {
            Section::Header => "Header",
            Section::Trailer => "Trailer",
        }
    }

    /// The message member that reaches the view.
    pub(crate) fn member_name(self) -> &'static str {
        match self {
            Section::Header => "header",
            Section::Trailer => "trailer",
        }
    }

    /// The context the section's own groups are declared in.
    pub(crate) fn context(self) -> MemberContext {
        match self {
            Section::Header => MemberContext::Header,
            Section::Trailer => MemberContext::Trailer,
        }
    }

    /// The section's members, components expanded in place.
    pub(crate) fn members(self, dictionary: &Dictionary) -> Vec<flat::Member<'_>> {
        flat::members(self.listed(dictionary).into_iter()).collect()
    }

    /// The section's members as the dictionary lists them.
    pub(crate) fn listed(self, dictionary: &Dictionary) -> Vec<dictionary::Member<'_>> {
        match self {
            Section::Header => dictionary.header().collect(),
            Section::Trailer => dictionary.trailer().collect(),
        }
    }
}

/// The sections a dictionary declares members for.
pub(crate) struct Envelope {
    pub(crate) sections: Vec<Section>,
    /// The sections holding a group. Their views keep the known tags they
    /// walk it with.
    pub(crate) with_groups: Vec<Section>,
}

impl Envelope {
    pub(crate) fn of(dictionary: &Dictionary) -> Self {
        let mut sections = Vec::new();
        let mut with_groups = Vec::new();
        for section in [Section::Header, Section::Trailer] {
            let members = section.members(dictionary);
            if members.is_empty() {
                continue;
            }
            sections.push(section);
            if members
                .iter()
                .any(|member| matches!(member, flat::Member::Group(_)))
            {
                with_groups.push(section);
            }
        }
        Self {
            sections,
            with_groups,
        }
    }
}
