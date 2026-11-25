use std::collections::{HashMap, HashSet};

use refix_dictionary::spec::Field;
use refix_dictionary::{Dictionary, MemberContext, Tag, dictionary};

use super::Error;
use super::naming::{
    component_property_name, group_property_name, member_name, nanos_declaration,
    nanos_property_name, numbered_member, property_name,
};
use crate::Warning;
use crate::components::components;
use crate::conversion::Conversion;
use crate::envelope::Section;
use crate::flat;
use crate::groups::{declared_groups, instance_context};
use crate::namespace::{NameClash, Namespace, Owner};
use crate::naming::group_source_name;

/// Names the generated module defines or relies on.
const MODULE: &[&str] = &[
    "CODEC",
    "ClassVar",
    "Codec",
    "DIALECT",
    "Dialect",
    "Header",
    "Message",
    "RawMessage",
    "TYPE_CHECKING",
    "Trailer",
    "_MESSAGES",
    "bytes",
    "dict",
    "enum",
    "fields",
    "from_raw",
    "refix",
    "type",
];

/// The members every message class defines or inherits, and `fields`, which
/// its body looks up.
///
/// An accessor named `fields` would replace the module for the declarations
/// after it.
const MESSAGE: &[&str] = &["MSG_TYPE", "fields", "from_raw", "header", "raw", "trailer"];

/// The members every header, trailer or component class inherits, and
/// `fields`, which its body looks up.
const VIEW: &[&str] = &["fields", "from_raw", "raw"];

/// The members every group instance class inherits, and `fields`, which its
/// body looks up.
const INSTANCE: &[&str] = &["fields", "from_raw", "raw"];

/// Every name the generated Python uses, settled before anything is emitted.
pub(super) struct Names {
    enums: HashMap<Tag, EnumNames>,
    properties: HashMap<Tag, String>,
    nanos_properties: HashMap<Tag, String>,
    groups: HashMap<MemberContext, GroupNames>,
    component_properties: HashMap<String, String>,
}

struct EnumNames {
    class: String,
    members: Vec<String>,
}

struct GroupNames {
    class: String,
    property: String,
}

impl Names {
    /// Names everything generated from `dictionary`.
    ///
    /// A clash that a rule settles is renamed with a warning. Any other
    /// clash is an error.
    pub(super) fn new(dictionary: &Dictionary, warnings: &mut Vec<Warning>) -> Result<Self, Error> {
        let mut names = Names {
            enums: HashMap::new(),
            properties: HashMap::new(),
            nanos_properties: HashMap::new(),
            groups: HashMap::new(),
            component_properties: HashMap::new(),
        };
        let mut module = Namespace::with_generated(MODULE);
        for message in dictionary.messages() {
            module.claim(message.name(), Owner::Message(message.name().to_owned()))?;
        }
        // A message's or section's own group class nests in its class, so only
        // a shared group's class is named in the module.
        for group in declared_groups(dictionary) {
            let class = names.name_group(group);
            if matches!(group.declared_in(), MemberContext::Component(_)) {
                module.claim(&class, Owner::Group(instance_context(group)))?;
            }
        }
        for component in components(dictionary) {
            let name = component.name();
            module.claim(name, Owner::Component(name.to_owned()))?;
            names
                .component_properties
                .insert(name.to_owned(), component_property_name(name));
        }
        for field in dictionary.fields() {
            if Conversion::of(field).is_enum() {
                let class = module.claim_enum(&field.name, warnings)?;
                names.name_enum(field, class, warnings)?;
            }
        }
        for message in dictionary.messages() {
            let mut members = Namespace::with_generated(MESSAGE);
            let used = names.classes_used(message.members());
            for member in flat::members(message.members()) {
                if let flat::Member::Group(group) = member
                    && matches!(group.declared_in(), MemberContext::Message(_))
                {
                    let class = names.group_class(group);
                    let owner = Owner::Group(instance_context(group));
                    hides_no_used_name(&module, &used, class, &owner)?;
                    members.claim(class, owner)?;
                }
            }
            names.name_properties(message.members(), members)?;
        }
        for section in [Section::Header, Section::Trailer] {
            names.name_section(section, dictionary, &module)?;
        }
        for group in declared_groups(dictionary) {
            let mut members = Namespace::with_generated(INSTANCE);
            let used = names.classes_used(group.members());
            for nested in nested_groups(group) {
                let class = names.group_class(nested);
                let owner = Owner::Group(instance_context(nested));
                hides_no_used_name(&module, &used, class, &owner)?;
                members.claim(class, owner)?;
            }
            names.name_properties(group.members(), members)?;
        }
        for component in components(dictionary) {
            let members = Namespace::with_generated(VIEW);
            names.name_properties(component.members(), members)?;
        }
        Ok(names)
    }

    /// Names a section's accessors beside the classes of the groups the
    /// section declares itself.
    fn name_section(
        &mut self,
        section: Section,
        dictionary: &Dictionary,
        module: &Namespace,
    ) -> Result<(), Error> {
        let members = section.members(dictionary);
        let mut namespace = Namespace::with_generated(VIEW);
        let used = self.classes_used(section.listed(dictionary).into_iter());
        for member in &members {
            if let flat::Member::Group(group) = member
                && *group.declared_in() == section.context()
            {
                let class = self.group_class(*group);
                let owner = Owner::Group(instance_context(*group));
                hides_no_used_name(module, &used, class, &owner)?;
                namespace.claim(class, owner)?;
            }
        }
        self.name_properties(section.listed(dictionary).into_iter(), namespace)
    }

    /// Names a group's class and property, returning the class.
    fn name_group(&mut self, group: dictionary::Group<'_>) -> String {
        let class = group_source_name(group).to_owned();
        let property = group_property_name(group);
        self.groups.insert(
            instance_context(group),
            GroupNames {
                class: class.clone(),
                property,
            },
        );
        class
    }

    fn name_enum(
        &mut self,
        field: &Field,
        class: String,
        warnings: &mut Vec<Warning>,
    ) -> Result<(), Error> {
        let mut members = Namespace::default();
        let names = field
            .values
            .iter()
            .map(|value| {
                let name = member_name(field, value)?;
                let owner = Owner::Value {
                    field: field.name.clone(),
                    code: value.value.clone(),
                    description: value.description.clone(),
                };
                Ok(members.claim_value(&name, owner, numbered_member, warnings)?)
            })
            .collect::<Result<Vec<_>, Error>>()?;
        self.enums.insert(
            field.tag,
            EnumNames {
                class,
                members: names,
            },
        );
        Ok(())
    }

    /// Names the properties of a message, group instance or component.
    fn name_properties<'a>(
        &mut self,
        members: impl Iterator<Item = dictionary::Member<'a>>,
        mut namespace: Namespace,
    ) -> Result<(), Error> {
        for member in members {
            match member {
                dictionary::Member::Field { field, .. } => {
                    let name = property_name(field);
                    namespace.claim(&name, Owner::Field(field.name.clone()))?;
                    self.properties.insert(field.tag, name);
                    if nanos_declaration(field).is_some() {
                        let name = nanos_property_name(field);
                        namespace.claim(&name, Owner::Nanos(field.name.clone()))?;
                        self.nanos_properties.insert(field.tag, name);
                    }
                }
                dictionary::Member::Group(group) => {
                    namespace.claim(
                        self.group_property(group),
                        Owner::Group(instance_context(group)),
                    )?;
                }
                dictionary::Member::Component(component) => {
                    let name = &self.component_properties[component.name()];
                    namespace.claim(name, Owner::Component(component.name().to_owned()))?;
                }
            }
        }
        Ok(())
    }

    /// The module classes that the declarations of `members` name.
    ///
    /// They are the enums of its fields, the classes of its shared groups
    /// and the views of its components.
    fn classes_used<'a>(
        &self,
        members: impl Iterator<Item = dictionary::Member<'a>>,
    ) -> HashSet<String> {
        members
            .filter_map(|member| match member {
                dictionary::Member::Field { field, .. } => {
                    self.enums.get(&field.tag).map(|names| names.class.clone())
                }
                dictionary::Member::Group(group) => {
                    matches!(group.declared_in(), MemberContext::Component(_))
                        .then(|| self.group_class(group).to_owned())
                }
                dictionary::Member::Component(component) => Some(component.name().to_owned()),
            })
            .collect()
    }

    /// The class of a field's enum.
    pub(super) fn enum_class(&self, field: &Field) -> &str {
        &self.enums[&field.tag].class
    }

    /// The class of a field's enum, for a field that has one.
    pub(super) fn enum_class_of(&self, field: &Field) -> Option<&str> {
        self.enums.get(&field.tag).map(|names| names.class.as_str())
    }

    /// The members of a field's enum, one per value.
    pub(super) fn enum_members(&self, field: &Field) -> &[String] {
        &self.enums[&field.tag].members
    }

    /// The property reading a field.
    pub(super) fn property(&self, field: &Field) -> &str {
        &self.properties[&field.tag]
    }

    /// The property reading a field's nanoseconds.
    pub(super) fn nanos_property(&self, field: &Field) -> &str {
        &self.nanos_properties[&field.tag]
    }

    /// A group's class.
    pub(super) fn group_class(&self, group: dictionary::Group<'_>) -> &str {
        &self.groups[&instance_context(group)].class
    }

    /// The property reading a group's instances.
    pub(super) fn group_property(&self, group: dictionary::Group<'_>) -> &str {
        &self.groups[&instance_context(group)].property
    }

    /// The property reading a component as a view.
    pub(super) fn component_property(&self, component: dictionary::Component<'_>) -> &str {
        &self.component_properties[component.name()]
    }
}

/// Fails if `class`, defined in the body of another class, is a module name
/// that body uses.
///
/// The declarations in a body name the classes in `used` without a path,
/// and the class would hide the one of its name. It may not hide a name the
/// generated code keeps for itself either.
fn hides_no_used_name(
    module: &Namespace,
    used: &HashSet<String>,
    class: &str,
    owner: &Owner,
) -> Result<(), Error> {
    match module.owner(class) {
        Some(first) if *first == Owner::Generated || used.contains(class) => {
            Err(Box::new(NameClash {
                name: class.to_owned(),
                first: first.clone(),
                second: owner.clone(),
            })
            .into())
        }
        _ => Ok(()),
    }
}

/// The groups declared directly in the instances of `group`.
fn nested_groups<'a>(group: dictionary::Group<'a>) -> impl Iterator<Item = dictionary::Group<'a>> {
    flat::members(group.members()).filter_map(|member| match member {
        flat::Member::Group(nested)
            if matches!(nested.declared_in(), MemberContext::Group { .. }) =>
        {
            Some(nested)
        }
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::Names;
    use crate::components::components;
    use crate::python::Error;
    use crate::test_utils::message_with;
    use crate::{NameClash, Owner, Warning};
    use refix_dictionary::spec::{
        Category, Component, ComponentRef, DataType, EnumValue, Field, FieldRef, Group, Member,
        Message, Protocol, Spec, Version,
    };
    use refix_dictionary::{Dictionary, MemberContext, Tag};

    fn field(name: &str, tag: u32, data_type: DataType) -> Field {
        Field {
            name: name.to_owned(),
            tag: Tag(tag),
            data_type,
            length_tag: None,
            values: vec![],
        }
    }

    fn with_values(mut field: Field, values: &[(&str, &str)]) -> Field {
        field.values = values
            .iter()
            .map(|(value, description)| EnumValue {
                value: (*value).to_owned(),
                description: (*description).to_owned(),
            })
            .collect();
        field
    }

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

    fn message(name: &str, msg_type: &str, members: Vec<Member>) -> Message {
        Message {
            name: name.to_owned(),
            msg_type: msg_type.to_owned(),
            members,
            category: Category::App,
        }
    }

    fn dictionary(
        messages: Vec<Message>,
        fields: Vec<Field>,
        components: Vec<Component>,
    ) -> Dictionary {
        Spec {
            version: Version {
                protocol: Protocol::Fix,
                major: 5,
                minor: 0,
                service_pack: 2,
            },
            header: vec![],
            trailer: vec![],
            messages,
            fields,
            components,
        }
        .resolve()
        .unwrap()
        .dictionary
    }

    fn clash_in(dictionary: &Dictionary) -> NameClash {
        match Names::new(dictionary, &mut Vec::new()) {
            Err(Error::NameClash(clash)) => *clash,
            Err(other) => panic!("expected a name clash, found {other:?}"),
            Ok(_) => panic!("expected a name clash"),
        }
    }

    fn clash(fields: Vec<Field>) -> NameClash {
        clash_in(&message_with(fields))
    }

    #[test]
    fn an_enum_named_like_a_message_yields() {
        let dictionary = message_with(vec![with_values(
            field("NewOrderSingle", 9000, DataType::Char),
            &[("1", "FIRST")],
        )]);
        let mut warnings = Vec::new();

        let names = Names::new(&dictionary, &mut warnings).unwrap();

        assert_eq!(
            names.enum_class(&dictionary.fields()[0]),
            "NewOrderSingleEnum"
        );
        assert_eq!(
            warnings,
            vec![Warning::Renamed {
                clash: Box::new(NameClash {
                    name: "NewOrderSingle".to_owned(),
                    first: Owner::Message("NewOrderSingle".to_owned()),
                    second: Owner::Enum("NewOrderSingle".to_owned()),
                }),
                name: "NewOrderSingleEnum".to_owned(),
            }]
        );
    }

    #[test]
    fn an_enum_named_like_an_import_clashes() {
        let field = with_values(field("RawMessage", 9000, DataType::Char), &[("1", "FIRST")]);
        assert_eq!(clash(vec![field]).first, Owner::Generated);
    }

    #[test]
    fn an_enum_named_like_a_shared_group_yields() {
        let dictionary = dictionary(
            vec![message(
                "MarketDataRequest",
                "V",
                vec![Member::Component(ComponentRef {
                    name: "RateSource".to_owned(),
                    is_required: false,
                })],
            )],
            vec![
                field("NoRateSources", 1445, DataType::NumInGroup),
                with_values(
                    field("RateSource", 1446, DataType::Int),
                    &[("0", "BLOOMBERG")],
                ),
            ],
            vec![Component {
                name: "RateSource".to_owned(),
                members: vec![group(1445, vec![field_ref(1446)])],
            }],
        );
        let mut warnings = Vec::new();

        let names = Names::new(&dictionary, &mut warnings).unwrap();

        assert_eq!(names.enum_class(&dictionary.fields()[1]), "RateSourceEnum");
        assert_eq!(
            warnings,
            vec![Warning::Renamed {
                clash: Box::new(NameClash {
                    name: "RateSource".to_owned(),
                    first: Owner::Group(
                        MemberContext::Component("RateSource".to_owned()).group("NoRateSources")
                    ),
                    second: Owner::Enum("RateSource".to_owned()),
                }),
                name: "RateSourceEnum".to_owned(),
            }]
        );
    }

    #[test]
    fn messages_may_nest_groups_of_one_name() {
        let routing = || group(215, vec![field_ref(216)]);
        let dictionary = dictionary(
            vec![
                message("IOI", "6", vec![routing()]),
                message("News", "B", vec![routing()]),
            ],
            vec![
                field("NoRoutingIDs", 215, DataType::NumInGroup),
                field("RoutingType", 216, DataType::Int),
            ],
            vec![],
        );
        assert!(Names::new(&dictionary, &mut Vec::new()).is_ok());
    }

    /// A message whose `NoMsgTypes` group declares the group counted by
    /// `nested_count` in its instances.
    fn with_a_group_in_a_group(nested_count: &str) -> Dictionary {
        dictionary(
            vec![message(
                "Logon",
                "A",
                vec![group(
                    384,
                    vec![field_ref(372), group(9001, vec![field_ref(9002)])],
                )],
            )],
            vec![
                field("NoMsgTypes", 384, DataType::NumInGroup),
                field("RefMsgType", 372, DataType::String),
                field(nested_count, 9001, DataType::NumInGroup),
                field("MsgSubType", 9002, DataType::String),
            ],
            vec![],
        )
    }

    fn nested_group(nested_count: &str) -> Owner {
        Owner::Group(
            MemberContext::Message("Logon".to_owned())
                .group("NoMsgTypes")
                .group(nested_count),
        )
    }

    #[test]
    fn a_nested_group_class_named_like_a_generated_name_clashes() {
        assert_eq!(
            clash_in(&with_a_group_in_a_group("NoRawMessage")),
            NameClash {
                name: "RawMessage".to_owned(),
                first: Owner::Generated,
                second: nested_group("NoRawMessage"),
            }
        );
    }

    #[test]
    fn a_group_class_may_be_named_like_a_message() {
        let dictionary = dictionary(
            vec![message(
                "SecurityTypes",
                "w",
                vec![group(558, vec![field_ref(167)])],
            )],
            vec![
                field("NoSecurityTypes", 558, DataType::NumInGroup),
                field("SecurityType", 167, DataType::String),
            ],
            vec![],
        );

        assert!(Names::new(&dictionary, &mut Vec::new()).is_ok());
    }

    #[test]
    fn a_group_class_may_be_named_like_the_enum_of_its_own_field() {
        let dictionary = dictionary(
            vec![message(
                "CollateralInquiry",
                "BB",
                vec![group(938, vec![field_ref(896)])],
            )],
            vec![
                field("NoCollInquiryQualifier", 938, DataType::NumInGroup),
                with_values(
                    field("CollInquiryQualifier", 896, DataType::Int),
                    &[("0", "TRADEDATE")],
                ),
            ],
            vec![],
        );

        assert!(Names::new(&dictionary, &mut Vec::new()).is_ok());
    }

    #[test]
    fn a_nested_group_class_named_like_an_enum_beside_it_clashes() {
        let dictionary = dictionary(
            vec![message(
                "Logon",
                "A",
                vec![group(
                    384,
                    vec![
                        field_ref(372),
                        field_ref(9003),
                        group(9001, vec![field_ref(9002)]),
                    ],
                )],
            )],
            vec![
                field("NoMsgTypes", 384, DataType::NumInGroup),
                field("RefMsgType", 372, DataType::String),
                field("NoSubTypes", 9001, DataType::NumInGroup),
                field("MsgSubType", 9002, DataType::String),
                with_values(field("SubTypes", 9003, DataType::Char), &[("1", "FIRST")]),
            ],
            vec![],
        );

        assert_eq!(
            clash_in(&dictionary),
            NameClash {
                name: "SubTypes".to_owned(),
                first: Owner::Enum("SubTypes".to_owned()),
                second: nested_group("NoSubTypes"),
            }
        );
    }

    #[test]
    fn a_message_group_class_named_like_an_enum_beside_it_clashes() {
        let dictionary = dictionary(
            vec![message(
                "Logon",
                "A",
                vec![field_ref(9000), group(384, vec![field_ref(372)])],
            )],
            vec![
                field("NoMsgTypes", 384, DataType::NumInGroup),
                field("RefMsgType", 372, DataType::String),
                with_values(field("MsgTypes", 9000, DataType::Char), &[("1", "FIRST")]),
            ],
            vec![],
        );

        assert_eq!(
            clash_in(&dictionary),
            NameClash {
                name: "MsgTypes".to_owned(),
                first: Owner::Enum("MsgTypes".to_owned()),
                second: Owner::Group(
                    MemberContext::Message("Logon".to_owned()).group("NoMsgTypes")
                ),
            }
        );
    }

    #[test]
    fn two_members_named_alike_are_numbered() {
        let dictionary = message_with(vec![with_values(
            field("BenchmarkCurveName", 221, DataType::String),
            &[("Euribor", "EURIBOR"), ("EURIBOR", "EURIBOR")],
        )]);
        let mut warnings = Vec::new();

        let names = Names::new(&dictionary, &mut warnings).unwrap();

        assert_eq!(
            names.enum_members(&dictionary.fields()[0]),
            ["EURIBOR", "EURIBOR_2"]
        );
        assert!(matches!(warnings.as_slice(), [Warning::Renamed { .. }]));
    }

    #[test]
    fn two_fields_with_one_property_name_clash() {
        assert_eq!(
            clash(vec![
                field("OrderQty", 38, DataType::Int),
                field("OrderQTY", 9000, DataType::Int),
            ]),
            NameClash {
                name: "order_qty".to_owned(),
                first: Owner::Field("OrderQty".to_owned()),
                second: Owner::Field("OrderQTY".to_owned()),
            }
        );
    }

    #[test]
    fn a_group_named_like_a_field_property_clashes() {
        let dictionary = dictionary(
            vec![message(
                "NewOrderSingle",
                "D",
                vec![
                    field_ref(9000),
                    Member::Component(ComponentRef {
                        name: "Parties".to_owned(),
                        is_required: false,
                    }),
                ],
            )],
            vec![
                field("PartyIDs", 9000, DataType::String),
                field("NoPartyIDs", 453, DataType::NumInGroup),
                field("PartyID", 448, DataType::String),
            ],
            vec![Component {
                name: "Parties".to_owned(),
                members: vec![group(453, vec![field_ref(448)])],
            }],
        );
        assert_eq!(
            clash_in(&dictionary),
            NameClash {
                name: "party_ids".to_owned(),
                first: Owner::Field("PartyIDs".to_owned()),
                second: Owner::Group(
                    MemberContext::Component("Parties".to_owned()).group("NoPartyIDs")
                ),
            }
        );
    }

    /// A dictionary whose `component` holds `Symbol`.
    fn with_component(
        component: &str,
        messages: Vec<Message>,
        mut fields: Vec<Field>,
    ) -> Dictionary {
        fields.push(field("Symbol", 55, DataType::String));
        let components = vec![Component {
            name: component.to_owned(),
            members: vec![field_ref(55)],
        }];
        dictionary(messages, fields, components)
    }

    fn component_ref(name: &str) -> Member {
        Member::Component(ComponentRef {
            name: name.to_owned(),
            is_required: false,
        })
    }

    fn order_with(members: Vec<Member>) -> Vec<Message> {
        vec![message("NewOrderSingle", "D", members)]
    }

    #[test]
    fn a_view_property_is_the_component_in_snake_case() {
        let messages = order_with(vec![component_ref("OrderQtyData")]);
        let dictionary = with_component("OrderQtyData", messages, vec![]);
        let names = Names::new(&dictionary, &mut Vec::new()).unwrap();
        let order_qty_data = components(&dictionary)[0];

        assert_eq!(names.component_property(order_qty_data), "order_qty_data");
    }

    #[test]
    fn a_view_property_named_like_a_field_property_clashes() {
        let messages = order_with(vec![field_ref(9000), component_ref("Instrument")]);
        let dictionary = with_component(
            "Instrument",
            messages,
            vec![field("Instrument", 9000, DataType::String)],
        );

        assert_eq!(
            clash_in(&dictionary),
            NameClash {
                name: "instrument".to_owned(),
                first: Owner::Field("Instrument".to_owned()),
                second: Owner::Component("Instrument".to_owned()),
            }
        );
    }

    #[test]
    fn a_component_may_hold_a_field_of_its_own_name() {
        let messages = order_with(vec![component_ref("Symbol")]);
        let dictionary = with_component("Symbol", messages, vec![]);
        let names = Names::new(&dictionary, &mut Vec::new()).unwrap();
        let symbol = components(&dictionary)[0];

        assert_eq!(names.component_property(symbol), "symbol");
        assert_eq!(names.property(&dictionary.fields()[0]), "symbol");
    }

    #[test]
    fn a_component_named_like_a_message_clashes() {
        let messages = vec![message("Quote", "S", vec![component_ref("Quote")])];
        let dictionary = with_component("Quote", messages, vec![]);

        assert_eq!(
            clash_in(&dictionary),
            NameClash {
                name: "Quote".to_owned(),
                first: Owner::Message("Quote".to_owned()),
                second: Owner::Component("Quote".to_owned()),
            }
        );
    }

    #[test]
    fn an_enum_named_like_a_component_yields() {
        let messages = order_with(vec![component_ref("Instrument")]);
        let dictionary = with_component(
            "Instrument",
            messages,
            vec![with_values(
                field("Instrument", 9000, DataType::Char),
                &[("1", "FIRST")],
            )],
        );
        let mut warnings = Vec::new();

        let names = Names::new(&dictionary, &mut warnings).unwrap();

        assert_eq!(names.enum_class(&dictionary.fields()[0]), "InstrumentEnum");
        assert_eq!(
            warnings,
            vec![Warning::Renamed {
                clash: Box::new(NameClash {
                    name: "Instrument".to_owned(),
                    first: Owner::Component("Instrument".to_owned()),
                    second: Owner::Enum("Instrument".to_owned()),
                }),
                name: "InstrumentEnum".to_owned(),
            }]
        );
    }

    #[test]
    fn a_field_named_like_a_nanosecond_property_clashes() {
        assert_eq!(
            clash(vec![
                field("MDEntryTime", 273, DataType::UtcTimeOnly),
                field("MDEntryTimeNs", 9000, DataType::Int),
            ]),
            NameClash {
                name: "md_entry_time_ns".to_owned(),
                first: Owner::Nanos("MDEntryTime".to_owned()),
                second: Owner::Field("MDEntryTimeNs".to_owned()),
            }
        );
    }

    #[test]
    fn a_property_named_like_a_generated_member_clashes() {
        assert_eq!(
            clash(vec![field("Raw", 9000, DataType::String)]).first,
            Owner::Generated
        );
    }

    #[test]
    fn a_property_named_like_the_header_view_clashes() {
        assert_eq!(
            clash(vec![field("Header", 9000, DataType::String)]),
            NameClash {
                name: "header".to_owned(),
                first: Owner::Generated,
                second: Owner::Field("Header".to_owned()),
            }
        );
    }

    #[test]
    fn a_property_named_like_the_fields_module_clashes() {
        assert_eq!(
            clash(vec![field("Fields", 9000, DataType::String)]),
            NameClash {
                name: "fields".to_owned(),
                first: Owner::Generated,
                second: Owner::Field("Fields".to_owned()),
            }
        );
    }

    #[test]
    fn a_clash_names_the_language() {
        let field = with_values(field("RawMessage", 9000, DataType::Char), &[("1", "FIRST")]);
        let error = Names::new(&message_with(vec![field]), &mut Vec::new())
            .err()
            .unwrap();
        assert_eq!(
            error.to_string(),
            "python name `RawMessage` is taken by both the generated code and the enum of field \
             'RawMessage'"
        );
    }
}
