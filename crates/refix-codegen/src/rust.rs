mod components;
mod dialect;
mod dispatch;
mod docs;
mod enums;
mod error;
mod groups;
mod layout;
mod messages;
mod modules;
mod names;
mod naming;
mod printing;
mod views;
mod warm_up;

use std::path::{Path, PathBuf};

use refix_dictionary::Dictionary;

use crate::components::components;
use crate::conversion::{Conversion, unclaimed_types};
use crate::dialect::length_tags;
use crate::envelope::Envelope;
use crate::groups::instance_context;
use crate::namespace::Owner;
use crate::package::MARKER;
use crate::{Module, Package};
use components::emit_component;
use dialect::emit_dialect;
use dispatch::emit_dispatch;
use enums::emit_enum;
pub use error::Error;
use groups::{emit_group, shared_groups};
use messages::emit_message;
use modules::{File, Folder, Modules, UNUSED, Uses, emit_folder, emit_imports, emit_use};
use names::Names;
use views::emit_view;
use warm_up::emit_warm_up;

/// The module tree's files.
///
/// The root file offers every type and reads a raw message as its type. The
/// enums share a file. Every message has a file in `messages` and every
/// other type has one in `components`, where a group declared in a type
/// stays with it. Only the root's names are public.
pub fn generate(dictionary: &Dictionary, source: &str) -> Result<Package, Error> {
    let mut warnings = Vec::new();
    let names = Names::new(dictionary, &mut warnings)?;
    let envelope = Envelope::of(dictionary);

    let enums: Vec<String> = dictionary
        .fields()
        .iter()
        .filter(|field| Conversion::of(field).is_enum())
        .map(|field| emit_enum(field, &names))
        .collect();
    let components = component_files(dictionary, &envelope, &names);
    let messages: Vec<File> = dictionary
        .messages()
        .map(|message| File {
            name: message.name().to_owned(),
            owner: Owner::Message(message.name().to_owned()),
            code: emit_message(message, &envelope, &names),
            uses: Uses::of_message(message, &envelope, &names),
        })
        .collect();

    // The layout is the emitter's own, so rustfmt leaves it alone. That keeps
    // the output byte-stable across rustfmt versions.
    let header =
        format!("// {MARKER} from {source}. Do not edit.\n#![cfg_attr(rustfmt, rustfmt::skip)]\n");
    let mut modules = Vec::new();
    let mut children = Vec::new();
    for (folder, files) in [
        (Folder::Components, &components),
        (Folder::Messages, &messages),
    ] {
        if files.is_empty() {
            continue;
        }
        let homes = Modules::new(files)?;
        let directory = Path::new(folder.name());
        modules.push(Module {
            path: directory.join("mod.rs"),
            code: format!("{header}\n{}", emit_folder(files, &homes)),
        });
        modules.extend(files.iter().map(|file| Module {
            path: directory.join(format!("{}.rs", homes.of(&file.name))),
            code: format!(
                "{header}\n{}\n{}",
                emit_imports(&file.uses, folder),
                file.code
            ),
        }));
        children.push(folder.name());
    }
    if !enums.is_empty() {
        modules.push(Module {
            path: PathBuf::from("enums.rs"),
            code: format!("{header}\n{}", enums.join("\n")),
        });
        children.push("enums");
    }
    children.sort_unstable();
    let sections = [emit_root_imports(dictionary), emit_children(&children)]
        .into_iter()
        .filter(|section| !section.is_empty())
        .chain([emit_dialect(dictionary)])
        .chain(emit_dispatch(dictionary))
        .chain(emit_warm_up(dictionary, &envelope, &names))
        .collect::<Vec<String>>()
        .join("\n");
    modules.insert(
        0,
        Module {
            path: PathBuf::from("mod.rs"),
            code: format!("{header}\n{sections}"),
        },
    );

    warnings.extend(unclaimed_types(dictionary));
    Ok(Package::new(
        modules,
        warnings,
        &["", Folder::Components.name(), Folder::Messages.name()],
    ))
}

/// The files in `components`, which hold the shared groups, the components
/// and the header and trailer views.
fn component_files(dictionary: &Dictionary, envelope: &Envelope, names: &Names) -> Vec<File> {
    let groups = shared_groups(dictionary).into_iter().map(|group| File {
        name: names.group_type(group).to_owned(),
        owner: Owner::Group(instance_context(group)),
        code: emit_group(group, names),
        uses: Uses::of_group(group, names),
    });
    let components = components(dictionary).into_iter().map(|component| File {
        name: component.name().to_owned(),
        owner: Owner::Component(component.name().to_owned()),
        code: emit_component(component, names),
        uses: Uses::of_component(component, names),
    });
    let views = envelope.sections.iter().map(|section| File {
        name: section.type_name().to_owned(),
        owner: Owner::Generated,
        code: emit_view(*section, dictionary, envelope, names),
        uses: Uses::of_view(*section, dictionary, envelope, names),
    });
    groups.chain(components).chain(views).collect()
}

/// What the root file's own items name from `refix_message`.
fn emit_root_imports(dictionary: &Dictionary) -> String {
    let has_messages = dictionary.messages().next().is_some();
    let mut imports = vec!["Dialect"];
    if has_messages {
        imports.push("RawMessage");
    }
    if has_messages || !length_tags(dictionary).is_empty() {
        imports.push("Tag");
    }
    emit_use("use", "refix_message", &imports)
}

/// The root's child modules, private, with everything in them offered from
/// the root.
fn emit_children(children: &[&str]) -> String {
    if children.is_empty() {
        return String::new();
    }
    let declarations: String = children
        .iter()
        .map(|child| format!("mod {child};\n"))
        .collect();
    let everything: Vec<String> = children.iter().map(|child| format!("{child}::*")).collect();
    let everything: Vec<&str> = everything.iter().map(String::as_str).collect();
    format!(
        "{declarations}\n{UNUSED}{}",
        emit_use("pub use", "self", &everything)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Warning;
    use refix_dictionary::Tag;
    use refix_dictionary::spec::{
        Category, DataType, EnumValue, Field, FieldRef, Group, Member, Message, Protocol, Spec,
        Version,
    };

    fn field(name: &str, tag: u32, data_type: DataType) -> Field {
        Field {
            name: name.to_owned(),
            tag: Tag(tag),
            data_type,
            length_tag: None,
            values: vec![],
        }
    }

    /// Every file's code, each under a line naming its path.
    fn listing(package: &Package) -> String {
        package
            .modules
            .iter()
            .map(|module| format!("=== {}\n{}", module.path.display(), module.code))
            .collect()
    }

    fn package_of(spec: Spec) -> Package {
        generate(&spec.resolve().unwrap().dictionary, "toy.xml").unwrap()
    }

    fn code_of(spec: Spec) -> String {
        listing(&package_of(spec))
    }

    /// The code of the file at `path`.
    fn file<'a>(package: &'a Package, path: &str) -> &'a str {
        let module = package
            .modules
            .iter()
            .find(|module| module.path == Path::new(path));
        &module.unwrap().code
    }

    fn spec_of(messages: Vec<Message>, fields: Vec<Field>) -> Spec {
        Spec {
            version: Version {
                protocol: Protocol::Fix,
                major: 4,
                minor: 4,
                service_pack: 0,
            },
            header: vec![],
            trailer: vec![],
            messages,
            components: vec![],
            fields,
        }
    }

    #[test]
    fn generates_a_complete_module_tree() {
        let mut ord_type = field("OrdType", 40, DataType::Char);
        ord_type.values = vec![
            EnumValue {
                value: "1".to_owned(),
                description: "MARKET".to_owned(),
            },
            EnumValue {
                value: "2".to_owned(),
                description: "LIMIT".to_owned(),
            },
        ];
        let spec = spec_of(
            vec![Message {
                name: "NewOrderSingle".to_owned(),
                msg_type: "D".to_owned(),
                members: vec![
                    Member::Field(FieldRef {
                        tag: Tag(11),
                        is_required: true,
                    }),
                    Member::Field(FieldRef {
                        tag: Tag(38),
                        is_required: false,
                    }),
                    Member::Field(FieldRef {
                        tag: Tag(44),
                        is_required: false,
                    }),
                    Member::Field(FieldRef {
                        tag: Tag(40),
                        is_required: false,
                    }),
                ],
                category: Category::App,
            }],
            vec![
                field("ClOrdID", 11, DataType::String),
                field("OrderQty", 38, DataType::Int),
                field("Price", 44, DataType::Other("PRICE".to_owned())),
                ord_type,
            ],
        );

        let expected = r#"=== mod.rs
// @generated by refix codegen from toy.xml. Do not edit.
#![cfg_attr(rustfmt, rustfmt::skip)]

use refix_message::{Dialect, RawMessage, Tag};

mod enums;
mod messages;

#[allow(unused_imports)]
pub use self::{enums::*, messages::*};

/// The dialect of this dictionary.
pub const DIALECT: Dialect<'static> = Dialect::new();

/// A message read as the type its MsgType names.
pub enum Message {
    NewOrderSingle(NewOrderSingle),
    Unrecognized(RawMessage),
}

impl Message {
    pub fn from_raw(raw: RawMessage) -> Self {
        match raw.get(Tag(35)) {
            Some(NewOrderSingle::MSG_TYPE) => Self::NewOrderSingle(NewOrderSingle::from_raw(raw)),
            _ => Self::Unrecognized(raw),
        }
    }
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewOrderSingle(message) => std::fmt::Debug::fmt(message, f),
            Self::Unrecognized(raw) => write!(f, "Unrecognized({raw})"),
        }
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewOrderSingle(message) => std::fmt::Display::fmt(message, f),
            Self::Unrecognized(raw) => std::fmt::Display::fmt(raw, f),
        }
    }
}
=== messages/mod.rs
// @generated by refix codegen from toy.xml. Do not edit.
#![cfg_attr(rustfmt, rustfmt::skip)]

mod new_order_single;

#[allow(unused_imports)]
pub use self::{
    new_order_single::NewOrderSingle,
};
=== messages/new_order_single.rs
// @generated by refix codegen from toy.xml. Do not edit.
#![cfg_attr(rustfmt, rustfmt::skip)]

use refix_message::{InvalidValue, RawMessage, Tag};

use super::super::enums::OrdType;

/// The `NewOrderSingle` (35=D) message.
pub struct NewOrderSingle(RawMessage);

impl NewOrderSingle {
    pub const MSG_TYPE: &[u8] = b"D";

    pub fn from_raw(raw: RawMessage) -> Self {
        Self(raw)
    }

    pub fn raw(&self) -> &RawMessage {
        &self.0
    }

    /// The `ClOrdID` (11) field.
    #[doc(alias = "ClOrdID")]
    pub fn cl_ord_id(&self) -> Result<Option<&str>, InvalidValue> {
        self.0.get_str(Tag(11))
    }

    /// The `OrderQty` (38) field.
    #[doc(alias = "OrderQty")]
    pub fn order_qty(&self) -> Result<Option<i64>, InvalidValue> {
        self.0.get_int(Tag(38))
    }

    /// The `Price` (44) field.
    #[doc(alias = "Price")]
    pub fn price_raw(&self) -> Option<&[u8]> {
        self.0.get(Tag(44))
    }

    /// The `OrdType` (40) field.
    #[doc(alias = "OrdType")]
    pub fn ord_type(&self) -> Result<Option<OrdType<'_>>, InvalidValue> {
        Ok(self.0.get_str(Tag(40))?.map(OrdType::from_value))
    }
}

impl std::fmt::Debug for NewOrderSingle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NewOrderSingle({})", self.0)
    }
}

impl std::fmt::Display for NewOrderSingle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}
=== enums.rs
// @generated by refix codegen from toy.xml. Do not edit.
#![cfg_attr(rustfmt, rustfmt::skip)]

/// The values of the `OrdType` (40) field.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum OrdType<'a> {
    /// `1`
    Market,
    /// `2`
    Limit,
    /// A value the dictionary does not list.
    Unrecognized(&'a str),
}

impl<'a> OrdType<'a> {
    pub fn from_value(value: &'a str) -> Self {
        match value {
            "1" => Self::Market,
            "2" => Self::Limit,
            unrecognized => Self::Unrecognized(unrecognized),
        }
    }

    pub fn value(self) -> &'a str {
        match self {
            Self::Market => "1",
            Self::Limit => "2",
            Self::Unrecognized(value) => value,
        }
    }
}

impl<'a> From<&'a str> for OrdType<'a> {
    fn from(value: &'a str) -> Self {
        Self::from_value(value)
    }
}
"#;

        let generated = package_of(spec);
        assert_eq!(listing(&generated), expected);
        assert_eq!(
            generated.warnings,
            [Warning::UnclaimedType {
                field: "Price".to_owned(),
                data_type: DataType::Other("PRICE".to_owned()),
            }]
        );
    }

    #[test]
    fn a_dictionary_without_types_is_one_file() {
        let package = package_of(spec_of(vec![], vec![]));

        assert_eq!(
            listing(&package),
            r#"=== mod.rs
// @generated by refix codegen from toy.xml. Do not edit.
#![cfg_attr(rustfmt, rustfmt::skip)]

use refix_message::Dialect;

/// The dialect of this dictionary.
pub const DIALECT: Dialect<'static> = Dialect::new();
"#
        );
    }

    #[test]
    fn two_types_needing_one_file_are_an_error() {
        let message = |name: &str, msg_type: &str| Message {
            name: name.to_owned(),
            msg_type: msg_type.to_owned(),
            members: vec![],
            category: Category::App,
        };
        let spec = spec_of(
            vec![message("QuoteID", "a"), message("QuoteId", "b")],
            vec![],
        );

        let error = generate(&spec.resolve().unwrap().dictionary, "toy.xml").unwrap_err();

        assert_eq!(
            error.to_string(),
            "rust module `quote_id` is taken by both message 'QuoteID' and message 'QuoteId'"
        );
    }

    #[test]
    fn a_dictionary_without_messages_has_no_message_enum() {
        let spec = spec_of(vec![], vec![]);

        let code = code_of(spec);

        assert!(!code.contains("pub enum Message"));
    }

    #[test]
    fn the_dialect_lists_every_linked_length_tag_in_order() {
        let spec = spec_of(
            vec![],
            vec![
                field("VenueDataLen", 5001, DataType::Length),
                Field {
                    length_tag: Some(Tag(5001)),
                    ..field("VenueData", 5002, DataType::Data)
                },
                field("RawDataLength", 95, DataType::Length),
                Field {
                    length_tag: Some(Tag(95)),
                    ..field("RawData", 96, DataType::Data)
                },
            ],
        );

        let code = code_of(spec);

        assert!(code.contains(
            "pub const DIALECT: Dialect<'static> = Dialect::new().with_extra_length_tags(&[Tag(95), Tag(5001)]);\n"
        ));
    }

    #[test]
    fn a_file_imports_multiple_values_only_when_it_reads_them() {
        let mut exec_inst = field("ExecInst", 18, DataType::MultipleStringValue);
        exec_inst.values = vec![EnumValue {
            value: "1".to_owned(),
            description: "NOT_HELD".to_owned(),
        }];
        let order = |members| Message {
            name: "NewOrderSingle".to_owned(),
            msg_type: "D".to_owned(),
            members,
            category: Category::App,
        };
        let reads = spec_of(
            vec![order(vec![Member::Field(FieldRef {
                tag: Tag(18),
                is_required: false,
            })])],
            vec![exec_inst.clone()],
        );
        let unused = spec_of(vec![order(vec![])], vec![exec_inst]);

        let imports = |spec: Spec| {
            let package = package_of(spec);
            let code = file(&package, "messages/new_order_single.rs");
            code.lines()
                .find(|line| line.starts_with("use "))
                .unwrap()
                .to_owned()
        };

        assert_eq!(
            imports(reads),
            "use refix_message::{InvalidValue, MultipleValues, RawMessage, Tag};"
        );
        assert_eq!(imports(unused), "use refix_message::RawMessage;");
    }

    #[test]
    fn a_file_imports_a_value_type_only_when_it_reads_one() {
        let order = |members| Message {
            name: "NewOrderSingle".to_owned(),
            msg_type: "D".to_owned(),
            members,
            category: Category::App,
        };
        let imports = |spec: Spec| {
            let package = package_of(spec);
            let code = file(&package, "messages/new_order_single.rs");
            code.lines()
                .find(|line| line.starts_with("use "))
                .unwrap()
                .to_owned()
        };

        for (value, expected) in [
            (
                field("Price", 44, DataType::Price),
                "use refix_message::{Decimal, InvalidValue, RawMessage, Tag};",
            ),
            (
                field("SettlDate", 64, DataType::LocalMktDate),
                "use refix_message::{Date, InvalidValue, RawMessage, Tag};",
            ),
            (
                field("MaturityMonthYear", 200, DataType::MonthYear),
                "use refix_message::{InvalidValue, MonthYear, RawMessage, Tag};",
            ),
            (
                field("MDEntryTime", 273, DataType::UtcTimeOnly),
                "use refix_message::{InvalidValue, RawMessage, Tag, Time};",
            ),
            (
                field("TransactTime", 60, DataType::UtcTimestamp),
                "use refix_message::{InvalidValue, RawMessage, Tag, UtcTimestamp};",
            ),
            (
                field("DeskOrderHandlingInst", 1035, DataType::MultipleStringValue),
                "use refix_message::{InvalidValue, MultipleValues, RawMessage, Tag};",
            ),
        ] {
            let reads = spec_of(
                vec![order(vec![Member::Field(FieldRef {
                    tag: value.tag,
                    is_required: false,
                })])],
                vec![value.clone()],
            );
            let unused = spec_of(vec![order(vec![])], vec![value]);

            assert_eq!(imports(reads), expected);
            assert_eq!(imports(unused), "use refix_message::RawMessage;");
        }
    }

    #[test]
    fn a_boolean_reads_as_a_bool_without_an_enum() {
        let mut locate_reqd = field("LocateReqd", 114, DataType::Boolean);
        locate_reqd.values = [("Y", "YES"), ("N", "NO")]
            .map(|(value, description)| EnumValue {
                value: value.to_owned(),
                description: description.to_owned(),
            })
            .to_vec();
        let spec = spec_of(
            vec![Message {
                name: "NewOrderSingle".to_owned(),
                msg_type: "D".to_owned(),
                members: vec![Member::Field(FieldRef {
                    tag: Tag(114),
                    is_required: false,
                })],
                category: Category::App,
            }],
            vec![locate_reqd],
        );

        let code = code_of(spec);

        assert!(code.contains("pub fn locate_reqd(&self) -> Result<Option<bool>, InvalidValue> {"));
        assert!(!code.contains("enum LocateReqd"));
    }

    #[test]
    fn an_enum_named_like_a_message_is_renamed_with_a_warning() {
        let mut security_status = field("SecurityStatus", 965, DataType::String);
        security_status.values = vec![EnumValue {
            value: "1".to_owned(),
            description: "ACTIVE".to_owned(),
        }];
        let spec = spec_of(
            vec![Message {
                name: "SecurityStatus".to_owned(),
                msg_type: "f".to_owned(),
                members: vec![Member::Field(FieldRef {
                    tag: Tag(965),
                    is_required: false,
                })],
                category: Category::App,
            }],
            vec![security_status],
        );

        let generated = package_of(spec);

        assert!(listing(&generated).contains("pub enum SecurityStatusEnum<'a> {"));
        assert!(listing(&generated).contains("pub struct SecurityStatus(RawMessage);"));
        assert!(
            listing(&generated).contains("Result<Option<SecurityStatusEnum<'_>>, InvalidValue>")
        );
        assert!(matches!(
            generated.warnings.as_slice(),
            [Warning::Renamed { .. }]
        ));
    }

    #[test]
    fn the_message_enum_lists_messages_in_spec_order() {
        let spec = spec_of(
            vec![
                Message {
                    name: "Heartbeat".to_owned(),
                    msg_type: "0".to_owned(),
                    members: vec![],
                    category: Category::Admin,
                },
                Message {
                    name: "TestRequest".to_owned(),
                    msg_type: "1".to_owned(),
                    members: vec![],
                    category: Category::Admin,
                },
            ],
            vec![],
        );

        let package = package_of(spec);

        assert!(file(&package, "mod.rs").contains(
            "pub enum Message {\n    Heartbeat(Heartbeat),\n    TestRequest(TestRequest),\n"
        ));
    }

    mod groups {
        use super::*;
        use refix_dictionary::spec::{Component, ComponentRef};

        fn field_ref(tag: u32) -> Member {
            Member::Field(FieldRef {
                tag: Tag(tag),
                is_required: false,
            })
        }

        fn component_ref(name: &str) -> Member {
            Member::Component(ComponentRef {
                name: name.to_owned(),
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

        /// A spec with `Parties` and its nested `PtysSubGrp`, as FIX44
        /// declares them, used by `messages`.
        fn parties_spec(messages: Vec<Message>) -> Spec {
            let mut spec = spec_of(
                messages,
                vec![
                    field("NoPartyIDs", 453, DataType::NumInGroup),
                    field("PartyID", 448, DataType::String),
                    field("NoPartySubIDs", 802, DataType::NumInGroup),
                    field("PartySubID", 523, DataType::String),
                ],
            );
            spec.components = vec![
                Component {
                    name: "Parties".to_owned(),
                    members: vec![group(
                        453,
                        vec![field_ref(448), component_ref("PtysSubGrp")],
                    )],
                },
                Component {
                    name: "PtysSubGrp".to_owned(),
                    members: vec![group(802, vec![field_ref(523)])],
                },
            ];
            spec
        }

        #[test]
        fn generates_a_type_per_group() {
            let spec = parties_spec(vec![message(
                "NewOrderSingle",
                "D",
                vec![component_ref("Parties")],
            )]);

            let expected = r#"=== mod.rs
// @generated by refix codegen from toy.xml. Do not edit.
#![cfg_attr(rustfmt, rustfmt::skip)]

use refix_message::{Dialect, RawMessage, Tag};

mod components;
mod messages;

#[allow(unused_imports)]
pub use self::{components::*, messages::*};

/// The dialect of this dictionary.
pub const DIALECT: Dialect<'static> = Dialect::new();

/// A message read as the type its MsgType names.
pub enum Message {
    NewOrderSingle(NewOrderSingle),
    Unrecognized(RawMessage),
}

impl Message {
    pub fn from_raw(raw: RawMessage) -> Self {
        match raw.get(Tag(35)) {
            Some(NewOrderSingle::MSG_TYPE) => Self::NewOrderSingle(NewOrderSingle::from_raw(raw)),
            _ => Self::Unrecognized(raw),
        }
    }
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewOrderSingle(message) => std::fmt::Debug::fmt(message, f),
            Self::Unrecognized(raw) => write!(f, "Unrecognized({raw})"),
        }
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewOrderSingle(message) => std::fmt::Display::fmt(message, f),
            Self::Unrecognized(raw) => std::fmt::Display::fmt(raw, f),
        }
    }
}

/// Builds the tag lists that group reads share.
///
/// Each list is otherwise built by the first read that needs it. Call this
/// once at startup to keep that work out of the first messages.
pub fn warm_up() {
    Parties::known_tags().get();
    PtysSubGrp::known_tags().get();
    NewOrderSingle::known_tags().get();
}
=== components/mod.rs
// @generated by refix codegen from toy.xml. Do not edit.
#![cfg_attr(rustfmt, rustfmt::skip)]

mod parties;
mod ptys_sub_grp;

#[allow(unused_imports)]
pub use self::{
    parties::Parties,
    ptys_sub_grp::PtysSubGrp,
};
=== components/parties.rs
// @generated by refix codegen from toy.xml. Do not edit.
#![cfg_attr(rustfmt, rustfmt::skip)]

use refix_message::{GroupTable, Instances, InvalidValue, LazyKnownTags, Tag};

use super::PtysSubGrp;

/// An instance of the `NoPartyIDs` (453) group.
#[derive(Clone, Copy)]
pub struct Parties<'a>(refix_message::Scope<'a>);

impl<'a> From<refix_message::Scope<'a>> for Parties<'a> {
    fn from(scope: refix_message::Scope<'a>) -> Self {
        Self(scope)
    }
}

impl<'a> Parties<'a> {
    pub(in super::super) const TABLE: GroupTable<'static> = GroupTable::new(
        Tag(453),
        Tag(448),
        &[Tag(448), Tag(802)],
        &[PtysSubGrp::TABLE],
    );

    pub(in super::super) const fn known_tags() -> &'static LazyKnownTags {
        static TAGS: LazyKnownTags = LazyKnownTags::new(
            &[Tag(448), Tag(802)],
            &[PtysSubGrp::known_tags()],
        );
        &TAGS
    }

    pub fn raw(&self) -> refix_message::Scope<'a> {
        self.0
    }

    /// The `PartyID` (448) field.
    #[doc(alias = "PartyID")]
    pub fn party_id(&self) -> Result<Option<&'a str>, InvalidValue> {
        self.0.get_str(Tag(448))
    }

    /// The instances of the `NoPartySubIDs` (802) group.
    #[doc(alias = "NoPartySubIDs")]
    pub fn party_sub_ids(&self) -> Result<Instances<'a, PtysSubGrp<'a>>, InvalidValue> {
        self.0.get_group(&PtysSubGrp::TABLE, &Self::known_tags().get()).map(Instances::from)
    }
}

impl std::fmt::Debug for Parties<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parties({})", self.0)
    }
}
=== components/ptys_sub_grp.rs
// @generated by refix codegen from toy.xml. Do not edit.
#![cfg_attr(rustfmt, rustfmt::skip)]

use refix_message::{GroupTable, InvalidValue, LazyKnownTags, Tag};

/// An instance of the `NoPartySubIDs` (802) group.
#[derive(Clone, Copy)]
pub struct PtysSubGrp<'a>(refix_message::Scope<'a>);

impl<'a> From<refix_message::Scope<'a>> for PtysSubGrp<'a> {
    fn from(scope: refix_message::Scope<'a>) -> Self {
        Self(scope)
    }
}

impl<'a> PtysSubGrp<'a> {
    pub(in super::super) const TABLE: GroupTable<'static> = GroupTable::new(
        Tag(802),
        Tag(523),
        &[Tag(523)],
        &[],
    );

    pub(in super::super) const fn known_tags() -> &'static LazyKnownTags {
        static TAGS: LazyKnownTags = LazyKnownTags::new(&[Tag(523)], &[]);
        &TAGS
    }

    pub fn raw(&self) -> refix_message::Scope<'a> {
        self.0
    }

    /// The `PartySubID` (523) field.
    #[doc(alias = "PartySubID")]
    pub fn party_sub_id(&self) -> Result<Option<&'a str>, InvalidValue> {
        self.0.get_str(Tag(523))
    }
}

impl std::fmt::Debug for PtysSubGrp<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PtysSubGrp({})", self.0)
    }
}
=== messages/mod.rs
// @generated by refix codegen from toy.xml. Do not edit.
#![cfg_attr(rustfmt, rustfmt::skip)]

mod new_order_single;

#[allow(unused_imports)]
pub use self::{
    new_order_single::NewOrderSingle,
};
=== messages/new_order_single.rs
// @generated by refix codegen from toy.xml. Do not edit.
#![cfg_attr(rustfmt, rustfmt::skip)]

use refix_message::{Instances, InvalidValue, LazyKnownTags, RawMessage, Tag};

use super::super::components::Parties;

/// The `NewOrderSingle` (35=D) message.
pub struct NewOrderSingle(RawMessage);

impl NewOrderSingle {
    pub const MSG_TYPE: &[u8] = b"D";

    pub(in super::super) const fn known_tags() -> &'static LazyKnownTags {
        static TAGS: LazyKnownTags = LazyKnownTags::new(&[Tag(453)], &[Parties::known_tags()]);
        &TAGS
    }

    pub fn from_raw(raw: RawMessage) -> Self {
        Self(raw)
    }

    pub fn raw(&self) -> &RawMessage {
        &self.0
    }

    /// The instances of the `NoPartyIDs` (453) group.
    #[doc(alias = "NoPartyIDs")]
    pub fn party_ids(&self) -> Result<Instances<'_, Parties<'_>>, InvalidValue> {
        self.0.get_group(&Parties::TABLE, &Self::known_tags().get()).map(Instances::from)
    }
}

impl std::fmt::Debug for NewOrderSingle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NewOrderSingle({})", self.0)
    }
}

impl std::fmt::Display for NewOrderSingle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}
"#;

            let generated = package_of(spec);
            assert_eq!(listing(&generated), expected);
            assert!(generated.warnings.is_empty());
        }

        #[test]
        fn a_shared_group_is_emitted_once() {
            let spec = parties_spec(vec![
                message("NewOrderSingle", "D", vec![component_ref("Parties")]),
                message("ExecutionReport", "8", vec![component_ref("Parties")]),
            ]);

            let code = code_of(spec);

            assert_eq!(code.matches("pub struct Parties<'a>").count(), 1);
            assert_eq!(code.matches("pub fn party_ids(&self)").count(), 2);
        }

        #[test]
        fn a_message_group_type_shares_the_message_file() {
            let spec = spec_of(
                vec![message(
                    "Logon",
                    "A",
                    vec![group(384, vec![field_ref(372)])],
                )],
                vec![
                    field("NoMsgTypes", 384, DataType::NumInGroup),
                    field("RefMsgType", 372, DataType::String),
                ],
            );

            let package = package_of(spec);

            let code = file(&package, "messages/logon.rs");
            assert!(code.contains("pub struct Logon(RawMessage);"));
            assert!(code.contains("pub struct LogonMsgTypes<'a>"));
            assert!(
                code.contains("self.0.get_group(&LogonMsgTypes::TABLE, &Self::known_tags().get())")
            );
            assert!(
                file(&package, "messages/mod.rs").contains("    logon::{Logon, LogonMsgTypes},\n")
            );
        }

        #[test]
        fn a_group_in_a_group_instance_gets_a_type_in_its_parents_file() {
            let mut spec = parties_spec(vec![message(
                "NewOrderSingle",
                "D",
                vec![component_ref("Parties")],
            )]);
            spec.components = vec![Component {
                name: "Parties".to_owned(),
                members: vec![group(
                    453,
                    vec![field_ref(448), group(802, vec![field_ref(523)])],
                )],
            }];

            let generated = package_of(spec);

            let code = file(&generated, "components/parties.rs");
            assert!(code.contains(
                "        &[Tag(448), Tag(802)],\n        &[PartiesPartySubIDs::TABLE],\n    );\n"
            ));
            assert!(code.contains(
                "    pub fn party_sub_ids(&self) -> Result<Instances<'a, PartiesPartySubIDs<'a>>, InvalidValue> {\n        self.0.get_group(&PartiesPartySubIDs::TABLE, &Self::known_tags().get()).map(Instances::from)\n    }\n"
            ));
            assert!(code.contains("pub struct Parties<'a>"));
            assert!(code.contains("pub struct PartiesPartySubIDs<'a>"));
            assert_eq!(generated.warnings, vec![]);
        }
    }
}
