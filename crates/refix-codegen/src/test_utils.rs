use refix_dictionary::{
    Category, Component, ComponentRef, DataType, Field, FieldRef, Group, Member, Message, Protocol,
    Spec, Tag, Version, dictionary,
};

/// Runs `read` on a group counted by `count_field`, declared inline in a
/// message or as the sole member of `component`.
pub fn with_group<R>(
    component: Option<&str>,
    count_field: &str,
    read: impl FnOnce(dictionary::Group<'_>) -> R,
) -> R {
    let group = Member::Group(Group {
        count_tag: Tag(453),
        is_required: false,
        members: vec![Member::Field(FieldRef {
            tag: Tag(448),
            is_required: false,
        })],
    });
    let (members, components) = match component {
        Some(name) => (
            vec![Member::Component(ComponentRef {
                name: name.to_owned(),
                is_required: false,
            })],
            vec![Component {
                name: name.to_owned(),
                members: vec![group],
            }],
        ),
        None => (vec![group], vec![]),
    };
    let field = |name: &str, tag: u32, data_type: DataType| Field {
        name: name.to_owned(),
        tag: Tag(tag),
        data_type,
        values: vec![],
    };
    let dictionary = Spec {
        version: Version {
            protocol: Protocol::Fix,
            major: 4,
            minor: 4,
            service_pack: 0,
        },
        messages: vec![Message {
            name: "NewOrderSingle".to_owned(),
            msg_type: "D".to_owned(),
            members,
            category: Category::App,
        }],
        fields: vec![
            field(count_field, 453, DataType::NumInGroup),
            field("PartyID", 448, DataType::String),
        ],
        components,
    }
    .resolve()
    .unwrap();
    let message = dictionary.messages().next().unwrap();
    match message.members().next() {
        Some(dictionary::Member::Group(group)) => read(group),
        other => panic!("expected a group, found {other:?}"),
    }
}
