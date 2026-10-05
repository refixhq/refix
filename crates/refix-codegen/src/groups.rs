use refix_dictionary::{MemberContext, Tag, dictionary};

/// Whether a group gets generated types. Groups declared directly in
/// another group's instance don't.
pub(crate) fn is_generated(group: dictionary::Group<'_>) -> bool {
    !matches!(group.declared_in(), MemberContext::Group { .. })
}

/// The tags a group's instances directly contain, sorted.
///
/// These are the instance's fields and the count tags of its nested groups.
pub(crate) fn member_tags(group: dictionary::Group<'_>) -> Vec<Tag> {
    let mut tags: Vec<Tag> = group
        .members()
        .map(|member| match member {
            dictionary::Member::Field { field, .. } => field.tag,
            dictionary::Member::Group(nested) => nested.count_field().tag,
        })
        .collect();
    tags.sort();
    tags
}

/// Every tag in `members` at any depth, group count tags included, sorted.
pub(crate) fn known_tags<'a>(members: impl Iterator<Item = dictionary::Member<'a>>) -> Vec<Tag> {
    let mut tags = Vec::new();
    collect_tags(members, &mut tags);
    tags.sort();
    tags
}

fn collect_tags<'a>(members: impl Iterator<Item = dictionary::Member<'a>>, tags: &mut Vec<Tag>) {
    for member in members {
        match member {
            dictionary::Member::Field { field, .. } => tags.push(field.tag),
            dictionary::Member::Group(group) => {
                tags.push(group.count_field().tag);
                collect_tags(group.members(), tags);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{is_generated, known_tags, member_tags};
    use refix_dictionary::{
        Category, DataType, Field, FieldRef, Group, Member, Message, Protocol, Spec, Tag, Version,
        dictionary,
    };

    fn field_ref(tag: u32) -> Member {
        Member::Field(FieldRef {
            tag: Tag(tag),
            is_required: false,
        })
    }

    fn group(count_tag: u32, members: Vec<Member>) -> Member {
        Member::Group(Group {
            count_tag: Tag(count_tag),
            is_required: false,
            members,
        })
    }

    /// NewOrderSingle with an inline NoPartyIDs whose instances hold an
    /// inline NoPartySubIDs.
    fn dictionary() -> refix_dictionary::Dictionary {
        let field = |name: &str, tag: u32, data_type: DataType| Field {
            name: name.to_owned(),
            tag: Tag(tag),
            data_type,
            values: vec![],
        };
        Spec {
            version: Version {
                protocol: Protocol::Fix,
                major: 4,
                minor: 4,
                service_pack: 0,
            },
            messages: vec![Message {
                name: "NewOrderSingle".to_owned(),
                msg_type: "D".to_owned(),
                members: vec![group(
                    453,
                    vec![
                        field_ref(452),
                        field_ref(448),
                        group(802, vec![field_ref(523)]),
                    ],
                )],
                category: Category::App,
            }],
            fields: vec![
                field("NoPartyIDs", 453, DataType::NumInGroup),
                field("PartyID", 448, DataType::String),
                field("PartyRole", 452, DataType::Int),
                field("NoPartySubIDs", 802, DataType::NumInGroup),
                field("PartySubID", 523, DataType::String),
            ],
            components: vec![],
        }
        .resolve()
        .unwrap()
    }

    fn groups<'a>(
        members: impl Iterator<Item = dictionary::Member<'a>>,
    ) -> Vec<dictionary::Group<'a>> {
        members
            .filter_map(|member| match member {
                dictionary::Member::Group(group) => Some(group),
                dictionary::Member::Field { .. } => None,
            })
            .collect()
    }

    #[test]
    fn member_tags_are_the_direct_tags_sorted() {
        let dictionary = dictionary();
        let message = dictionary.messages().next().unwrap();
        let parties = groups(message.members())[0];

        assert_eq!(member_tags(parties), [Tag(448), Tag(452), Tag(802)]);
    }

    #[test]
    fn known_tags_reach_every_depth() {
        let dictionary = dictionary();
        let message = dictionary.messages().next().unwrap();

        assert_eq!(
            known_tags(message.members()),
            [Tag(448), Tag(452), Tag(453), Tag(523), Tag(802)]
        );
    }

    #[test]
    fn a_group_declared_in_an_instance_is_not_generated() {
        let dictionary = dictionary();
        let message = dictionary.messages().next().unwrap();
        let parties = groups(message.members())[0];
        let sub_ids = groups(parties.members())[0];

        assert!(is_generated(parties));
        assert!(!is_generated(sub_ids));
    }
}
