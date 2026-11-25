mod components;
mod constructors;
mod declarations;
mod dialect;
mod dispatch;
mod enums;
mod error;
mod groups;
mod layout;
mod messages;
mod modules;
mod names;
mod naming;
mod runtime;
mod shared;
mod views;

use std::path::{Path, PathBuf};

use refix_dictionary::Dictionary;

use crate::conversion::{Conversion, unclaimed_types};
use crate::envelope::Envelope;
use crate::groups::instance_context;
use crate::namespace::Owner;
use crate::package::MARKER;
use crate::{Module, Package};
use components::emit_component;
use dialect::{emit_codec, emit_dialect};
use dispatch::emit_dispatch;
use enums::emit_enum;
pub use error::Error;
use groups::emit_group_class;
use messages::emit_message;
use modules::{Class, Modules, Subpackage, Uses, emit_from, sorted_names};
use names::Names;
use runtime::emit_runtime_check;
use shared::{Shared, shared_classes};
use views::emit_view;

/// The package's files.
///
/// The root module offers every class and reads a raw message as its type.
/// The enums share a module. Every message has a module in `messages` and
/// every other class has one in `components`. Both subpackages offer their
/// classes too.
pub fn generate(dictionary: &Dictionary, source: &str) -> Result<Package, Error> {
    let mut warnings = Vec::new();
    let names = Names::new(dictionary, &mut warnings)?;
    let envelope = Envelope::of(dictionary);

    let enums: Vec<(&str, String)> = dictionary
        .fields()
        .iter()
        .filter(|field| Conversion::of(field).is_enum())
        .map(|field| (names.enum_class(field), emit_enum(field, &names)))
        .collect();
    let components = component_classes(dictionary, &envelope, &names);
    let messages: Vec<Class> = dictionary
        .messages()
        .map(|message| {
            let mut uses = Uses::of(message.members(), &names);
            let views = envelope.sections.iter().map(|section| section.type_name());
            uses.classes.extend(views.map(str::to_owned));
            Class {
                name: message.name().to_owned(),
                owner: Owner::Message(message.name().to_owned()),
                code: emit_message(message, &envelope, &names),
                uses,
            }
        })
        .collect();
    let component_modules = Modules::new(&components)?;
    let message_modules = Modules::new(&messages)?;

    // The layout is the emitter's own, so formatters leave it alone.
    let header = format!("# {MARKER} from {source}. Do not edit.\n# fmt: off\n");
    let mut reexports: Vec<(Subpackage, String)> = Vec::new();
    let mut modules = Vec::new();
    for (subpackage, classes, homes) in [
        (Subpackage::Components, &components, &component_modules),
        (Subpackage::Messages, &messages, &message_modules),
    ] {
        if classes.is_empty() {
            continue;
        }
        let directory = Path::new(subpackage.name());
        let mut offered = Vec::new();
        let mut class_modules = Vec::new();
        for class in classes {
            let module = homes.of(&class.name);
            let imports = modules::emit_imports(class, subpackage, &component_modules);
            class_modules.push(Module {
                path: directory.join(format!("{module}.py")),
                code: format!("{header}\n{imports}\n\n{}", class.code),
            });
            let name = format!("{name} as {name}", name = class.name);
            offered.push((format!(".{module}"), name));
        }
        offered.sort();
        let from_modules: String = offered
            .iter()
            .map(|(module, name)| emit_from(module, &[name]))
            .collect();
        modules.push(Module {
            path: directory.join("__init__.py"),
            code: format!("{header}\n{from_modules}"),
        });
        modules.extend(class_modules);
        let from_subpackage = format!(".{}", subpackage.name());
        let names: Vec<&str> = classes.iter().map(|class| class.name.as_str()).collect();
        let lines: String = sorted_names(names.into_iter())
            .iter()
            .map(|name| emit_from(&from_subpackage, &[&format!("{name} as {name}")]))
            .collect();
        reexports.push((subpackage, lines));
    }
    if !enums.is_empty() {
        let classes: Vec<&str> = enums.iter().map(|(_, code)| code.as_str()).collect();
        modules.push(Module {
            path: PathBuf::from("enums.py"),
            code: format!("{header}\nimport enum\n\n\n{}", classes.join("\n\n")),
        });
    }
    let enum_reexports: String = sorted_names(enums.iter().map(|(name, _)| *name))
        .iter()
        .map(|name| emit_from(".enums", &[&format!("{name} as {name}")]))
        .collect();
    // Modules sort as isort has them, so the enums sit between the two.
    let relative: String = reexports
        .iter()
        .filter(|(subpackage, _)| *subpackage == Subpackage::Components)
        .map(|(_, lines)| lines.as_str())
        .chain([enum_reexports.as_str()])
        .chain(
            reexports
                .iter()
                .filter(|(subpackage, _)| *subpackage == Subpackage::Messages)
                .map(|(_, lines)| lines.as_str()),
        )
        .collect();
    let imports = emit_root_imports(dictionary, &relative);
    // isort wants one blank line between the imports and a constant.
    let preamble = format!("{imports}\n{}", emit_dialect(dictionary));
    let sections = [preamble]
        .into_iter()
        .chain(emit_dispatch(dictionary))
        .chain(emit_codec(dictionary))
        .collect::<Vec<String>>()
        .join("\n\n");
    modules.insert(
        0,
        Module {
            path: PathBuf::from("__init__.py"),
            code: format!("{header}\n{sections}"),
        },
    );

    warnings.extend(unclaimed_types(dictionary));
    Ok(Package::new(
        modules,
        warnings,
        &[
            "",
            Subpackage::Components.name(),
            Subpackage::Messages.name(),
        ],
    ))
}

/// Every class in the `components` subpackage, a class after the classes
/// its declarations name.
fn component_classes(dictionary: &Dictionary, envelope: &Envelope, names: &Names) -> Vec<Class> {
    let shared = shared_classes(dictionary)
        .into_iter()
        .map(|class| match class {
            Shared::Group(group) => Class {
                name: names.group_class(group).to_owned(),
                owner: Owner::Group(instance_context(group)),
                code: emit_group_class(group, names),
                uses: Uses::of(group.members(), names),
            },
            Shared::Component(component) => Class {
                name: component.name().to_owned(),
                owner: Owner::Component(component.name().to_owned()),
                code: emit_component(component, names),
                uses: Uses::of(component.members(), names),
            },
        });
    let views = envelope.sections.iter().map(|section| Class {
        name: section.type_name().to_owned(),
        owner: Owner::Generated,
        code: emit_view(*section, dictionary, names),
        uses: Uses::of(section.listed(dictionary).into_iter(), names),
    });
    shared.chain(views).collect()
}

/// The root module's import block, with the runtime check ahead of the
/// names imported from `refix` and of every class the package offers.
fn emit_root_imports(dictionary: &Dictionary, relative: &str) -> String {
    let has_messages = dictionary.messages().next().is_some();
    let from_refix = if has_messages {
        "from refix import Codec, Dialect, RawMessage\n"
    } else {
        "from refix import Dialect\n"
    };
    let check = emit_runtime_check();
    let blocks = ["import refix\n", check.as_str(), from_refix, relative];
    let blocks: Vec<&str> = blocks
        .into_iter()
        .filter(|block| !block.is_empty())
        .collect();
    blocks.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Warning;
    use refix_dictionary::Tag;
    use refix_dictionary::spec::{
        Category, Component, ComponentRef, DataType, EnumValue, Field, FieldRef, Group, Member,
        Message, Protocol, Spec, Version,
    };

    fn field(name: &str, tag: u32, data_type: DataType, values: &[(&str, &str)]) -> Field {
        Field {
            name: name.to_owned(),
            tag: Tag(tag),
            data_type,
            length_tag: None,
            values: values
                .iter()
                .map(|(value, description)| EnumValue {
                    value: (*value).to_owned(),
                    description: (*description).to_owned(),
                })
                .collect(),
        }
    }

    fn field_ref(tag: u32) -> Member {
        Member::Field(FieldRef {
            tag: Tag(tag),
            is_required: false,
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

    fn generated(
        messages: Vec<Message>,
        fields: Vec<Field>,
        components: Vec<Component>,
    ) -> Package {
        let spec = Spec {
            version: Version {
                protocol: Protocol::Fix,
                major: 4,
                minor: 4,
                service_pack: 0,
            },
            header: vec![],
            trailer: vec![],
            messages,
            fields,
            components,
        };
        generate(&spec.resolve().unwrap().dictionary, "toy.xml").unwrap()
    }

    /// Every module's code, each under a line naming its path.
    fn listing(package: &Package) -> String {
        package
            .modules
            .iter()
            .map(|module| format!("=== {}\n{}", module.path.display(), module.code))
            .collect()
    }

    fn code(messages: Vec<Message>, fields: Vec<Field>, components: Vec<Component>) -> String {
        listing(&generated(messages, fields, components))
    }

    #[test]
    fn generates_a_complete_package() {
        let generated = generated(
            vec![message(
                "NewOrderSingle",
                "D",
                [11, 38, 44, 40, 452, 18].map(field_ref).to_vec(),
            )],
            vec![
                field("ClOrdID", 11, DataType::String, &[]),
                field("OrderQty", 38, DataType::Int, &[]),
                field("Price", 44, DataType::Other("PRICE".to_owned()), &[]),
                field(
                    "OrdType",
                    40,
                    DataType::Char,
                    &[("1", "MARKET"), ("2", "LIMIT")],
                ),
                field("PartyRole", 452, DataType::Int, &[("3", "CLIENT_ID")]),
                field(
                    "ExecInst",
                    18,
                    DataType::MultipleStringValue,
                    &[("1", "NOT_HELD")],
                ),
            ],
            vec![],
        );

        let expected = r#"=== __init__.py
# @generated by refix codegen from toy.xml. Do not edit.
# fmt: off

import refix

if tuple(map(int, refix.__version__.split(".")[:2])) < (0, 10):
    raise ImportError(
        f"{__name__} needs refix 0.10 or newer, but refix {refix.__version__} is installed"
    )

from refix import Codec, Dialect, RawMessage

from .enums import ExecInst as ExecInst
from .enums import OrdType as OrdType
from .enums import PartyRole as PartyRole
from .messages import NewOrderSingle as NewOrderSingle

DIALECT = Dialect()


type Message = NewOrderSingle


_MESSAGES: dict[bytes, type[Message]] = {
    NewOrderSingle.MSG_TYPE: NewOrderSingle,
}


def from_raw(raw: RawMessage) -> Message | RawMessage:
    msg_type = raw.get(35)
    message = None if msg_type is None else _MESSAGES.get(msg_type)
    return raw if message is None else message.from_raw(raw)


CODEC: Codec[Message | RawMessage] = Codec(DIALECT, from_raw)
=== messages/__init__.py
# @generated by refix codegen from toy.xml. Do not edit.
# fmt: off

from .new_order_single import NewOrderSingle as NewOrderSingle
=== messages/new_order_single.py
# @generated by refix codegen from toy.xml. Do not edit.
# fmt: off

from typing import TYPE_CHECKING, ClassVar

from refix import fields

from ..enums import ExecInst, OrdType, PartyRole


class NewOrderSingle(fields.Message):
    """The NewOrderSingle (35=D) message."""

    MSG_TYPE: ClassVar[bytes] = b"D"

    cl_ord_id = fields.Str(11)
    order_qty = fields.Int(38)
    price_raw = fields.Bytes(44)
    ord_type = fields.Str(40, OrdType)
    party_role = fields.Int(452, PartyRole)
    exec_inst = fields.MultipleValues(18, ExecInst)

    if TYPE_CHECKING:
        def __init__(self) -> None: ...
=== enums.py
# @generated by refix codegen from toy.xml. Do not edit.
# fmt: off

import enum


class OrdType(enum.StrEnum):
    MARKET = "1"
    LIMIT = "2"


class PartyRole(enum.IntEnum):
    CLIENT_ID = 3


class ExecInst(enum.StrEnum):
    NOT_HELD = "1"
"#;
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
    fn a_message_docstring_escapes_the_msg_type() {
        let generated = generated(vec![message("Quote", "\"", vec![])], vec![], vec![]);

        assert!(listing(&generated).contains(
            "class Quote(fields.Message):\n    \"\"\"The Quote (35=\\\") message.\"\"\"\n"
        ));
    }

    #[test]
    fn a_dictionary_without_messages_has_no_dispatch_or_codec() {
        let code = code(vec![], vec![], vec![]);

        assert!(!code.contains("type Message"));
        assert!(!code.contains("def from_raw"));
        assert!(!code.contains("Codec"));
    }

    #[test]
    fn the_dialect_lists_every_linked_length_tag_in_order() {
        let code = code(
            vec![],
            vec![
                field("VenueDataLen", 5001, DataType::Length, &[]),
                Field {
                    length_tag: Some(Tag(5001)),
                    ..field("VenueData", 5002, DataType::Data, &[])
                },
                field("RawDataLength", 95, DataType::Length, &[]),
                Field {
                    length_tag: Some(Tag(95)),
                    ..field("RawData", 96, DataType::Data, &[])
                },
            ],
            vec![],
        );

        assert!(code.contains("\nDIALECT = Dialect(extra_length_tags=(95, 5001))\n"));
    }

    #[test]
    fn a_long_message_alias_puts_a_class_per_line() {
        let code = code(
            vec![
                message("NetworkCounterpartySystemStatusRequest", "BC", vec![]),
                message("NetworkCounterpartySystemStatusResponse", "BD", vec![]),
                message("RegistrationInstructionsResponse", "p", vec![]),
            ],
            vec![],
            vec![],
        );

        assert!(code.contains(
            "type Message = (\n    NetworkCounterpartySystemStatusRequest\n    | NetworkCounterpartySystemStatusResponse\n    | RegistrationInstructionsResponse\n)\n"
        ));
    }

    #[test]
    fn an_unclaimed_type_warns_once_per_field_read() {
        let quantity = || DataType::Other("QUANTITY".to_owned());
        let generated = generated(
            vec![
                message("NewOrderSingle", "D", vec![field_ref(319)]),
                message("ExecutionReport", "8", vec![field_ref(319)]),
            ],
            vec![
                field("RatioQty", 319, quantity(), &[]),
                field("Unread", 9000, quantity(), &[]),
            ],
            vec![],
        );

        assert_eq!(
            generated.warnings,
            [Warning::UnclaimedType {
                field: "RatioQty".to_owned(),
                data_type: quantity(),
            }]
        );
    }

    #[test]
    fn a_char_reads_as_text() {
        let generated = generated(
            vec![message("NewOrderSingle", "D", vec![field_ref(206)])],
            vec![field("OptAttribute", 206, DataType::Char, &[])],
            vec![],
        );

        assert!(listing(&generated).contains("    opt_attribute = fields.Str(206)\n"));
    }

    #[test]
    fn a_boolean_reads_as_a_bool_without_an_enum() {
        let generated = generated(
            vec![message("NewOrderSingle", "D", vec![field_ref(114)])],
            vec![field(
                "LocateReqd",
                114,
                DataType::Boolean,
                &[("Y", "YES"), ("N", "NO")],
            )],
            vec![],
        );

        assert!(listing(&generated).contains("    locate_reqd = fields.Bool(114)\n"));
        assert!(!listing(&generated).contains("class LocateReqd"));
    }

    #[test]
    fn a_float_reads_as_a_decimal() {
        let generated = generated(
            vec![message("NewOrderSingle", "D", vec![field_ref(44)])],
            vec![field("Price", 44, DataType::Price, &[])],
            vec![],
        );

        assert!(listing(&generated).contains("    price = fields.Decimal(44)\n"));
    }

    #[test]
    fn a_date_reads_as_a_date() {
        let generated = generated(
            vec![message("NewOrderSingle", "D", vec![field_ref(64)])],
            vec![field("SettlDate", 64, DataType::LocalMktDate, &[])],
            vec![],
        );

        assert!(listing(&generated).contains("    settl_date = fields.Date(64)\n"));
    }

    #[test]
    fn a_utc_time_reads_as_a_time_beside_its_nanoseconds() {
        let generated = generated(
            vec![message("MarketDataSnapshot", "W", vec![field_ref(273)])],
            vec![field("MDEntryTime", 273, DataType::UtcTimeOnly, &[])],
            vec![],
        );

        assert!(listing(&generated).contains(
            "    md_entry_time = fields.Time(273)\n    md_entry_time_ns = fields.TimeNs(273)\n"
        ));
    }

    #[test]
    fn a_utc_timestamp_reads_as_a_datetime_beside_its_nanoseconds() {
        let generated = generated(
            vec![message("NewOrderSingle", "D", vec![field_ref(60)])],
            vec![field("TransactTime", 60, DataType::UtcTimestamp, &[])],
            vec![],
        );

        assert!(
            listing(&generated)
                .contains("    transact_time = fields.UtcTimestamp(60)\n    transact_time_ns = fields.UtcTimestampNs(60)\n")
        );
    }

    #[test]
    fn a_multiple_value_field_without_values_reads_text_values() {
        let generated = generated(
            vec![message("NewOrderSingle", "D", vec![field_ref(1035)])],
            vec![field(
                "DeskOrderHandlingInst",
                1035,
                DataType::MultipleStringValue,
                &[],
            )],
            vec![],
        );

        assert!(
            listing(&generated)
                .contains("    desk_order_handling_inst = fields.MultipleValues(1035)\n")
        );
    }

    #[test]
    fn a_data_field_reads_as_bytes() {
        let generated = generated(
            vec![message("Logon", "A", vec![field_ref(96)])],
            vec![
                field("RawDataLength", 95, DataType::Length, &[]),
                Field {
                    length_tag: Some(Tag(95)),
                    ..field("RawData", 96, DataType::Data, &[])
                },
            ],
            vec![],
        );

        assert!(listing(&generated).contains("    raw_data = fields.Bytes(96)\n"));
    }

    #[test]
    fn a_month_year_reads_as_a_month_year() {
        let generated = generated(
            vec![message("NewOrderSingle", "D", vec![field_ref(200)])],
            vec![field("MaturityMonthYear", 200, DataType::MonthYear, &[])],
            vec![],
        );

        assert!(listing(&generated).contains("    maturity_month_year = fields.MonthYear(200)\n"));
    }

    #[test]
    fn the_root_imports_only_what_it_uses() {
        let with_a_message = generated(vec![message("Logout", "5", vec![])], vec![], vec![]);
        let without_classes = generated(vec![], vec![], vec![]);

        let imports = |package: &Package| {
            let root = &package.modules[0];
            assert_eq!(root.path, Path::new("__init__.py"));
            root.code.split("\n\nDIALECT").next().unwrap().to_owned()
        };
        assert_eq!(
            imports(&with_a_message),
            r#"# @generated by refix codegen from toy.xml. Do not edit.
# fmt: off

import refix

if tuple(map(int, refix.__version__.split(".")[:2])) < (0, 10):
    raise ImportError(
        f"{__name__} needs refix 0.10 or newer, but refix {refix.__version__} is installed"
    )

from refix import Codec, Dialect, RawMessage

from .messages import Logout as Logout"#
        );
        assert_eq!(
            imports(&without_classes),
            r#"# @generated by refix codegen from toy.xml. Do not edit.
# fmt: off

import refix

if tuple(map(int, refix.__version__.split(".")[:2])) < (0, 10):
    raise ImportError(
        f"{__name__} needs refix 0.10 or newer, but refix {refix.__version__} is installed"
    )

from refix import Dialect"#
        );
    }

    #[test]
    fn a_dictionary_without_classes_is_one_module() {
        let package = generated(vec![], vec![], vec![]);

        let paths: Vec<&Path> = package
            .modules
            .iter()
            .map(|module| module.path.as_path())
            .collect();
        assert_eq!(paths, [Path::new("__init__.py")]);
    }

    #[test]
    fn two_classes_needing_one_module_are_an_error() {
        let spec = Spec {
            version: Version {
                protocol: Protocol::Fix,
                major: 4,
                minor: 4,
                service_pack: 0,
            },
            header: vec![],
            trailer: vec![],
            messages: vec![
                message("QuoteID", "a", vec![]),
                message("QuoteId", "b", vec![]),
            ],
            fields: vec![],
            components: vec![],
        };

        let error = generate(&spec.resolve().unwrap().dictionary, "toy.xml").unwrap_err();

        assert_eq!(
            error.to_string(),
            "python module `quote_id` is taken by both message 'QuoteID' and message 'QuoteId'"
        );
    }

    fn group(count_tag: u32, members: Vec<Member>) -> Member {
        Member::Group(Group {
            count_tag: Tag(count_tag),
            is_required: false,
            members,
        })
    }

    fn component_ref(name: &str) -> Member {
        Member::Component(ComponentRef {
            name: name.to_owned(),
            is_required: false,
        })
    }

    fn component(name: &str, members: Vec<Member>) -> Component {
        Component {
            name: name.to_owned(),
            members,
        }
    }

    fn party_fields() -> Vec<Field> {
        vec![
            field("NoPartyIDs", 453, DataType::NumInGroup, &[]),
            field("PartyID", 448, DataType::String, &[]),
            field("NoPartySubIDs", 802, DataType::NumInGroup, &[]),
            field("PartySubID", 523, DataType::String, &[]),
        ]
    }

    /// `Parties` and its nested `PtysSubGrp`, as FIX44 declares them.
    fn party_components() -> Vec<Component> {
        vec![
            component(
                "Parties",
                vec![group(
                    453,
                    vec![field_ref(448), component_ref("PtysSubGrp")],
                )],
            ),
            component("PtysSubGrp", vec![group(802, vec![field_ref(523)])]),
        ]
    }

    #[test]
    fn a_shared_group_class_gets_a_module() {
        let generated = generated(
            vec![message(
                "NewOrderSingle",
                "D",
                vec![component_ref("Parties")],
            )],
            party_fields(),
            party_components(),
        );

        let expected = r#"=== __init__.py
# @generated by refix codegen from toy.xml. Do not edit.
# fmt: off

import refix

if tuple(map(int, refix.__version__.split(".")[:2])) < (0, 10):
    raise ImportError(
        f"{__name__} needs refix 0.10 or newer, but refix {refix.__version__} is installed"
    )

from refix import Codec, Dialect, RawMessage

from .components import Parties as Parties
from .components import PtysSubGrp as PtysSubGrp
from .messages import NewOrderSingle as NewOrderSingle

DIALECT = Dialect()


type Message = NewOrderSingle


_MESSAGES: dict[bytes, type[Message]] = {
    NewOrderSingle.MSG_TYPE: NewOrderSingle,
}


def from_raw(raw: RawMessage) -> Message | RawMessage:
    msg_type = raw.get(35)
    message = None if msg_type is None else _MESSAGES.get(msg_type)
    return raw if message is None else message.from_raw(raw)


CODEC: Codec[Message | RawMessage] = Codec(DIALECT, from_raw)
=== components/__init__.py
# @generated by refix codegen from toy.xml. Do not edit.
# fmt: off

from .parties import Parties as Parties
from .ptys_sub_grp import PtysSubGrp as PtysSubGrp
=== components/ptys_sub_grp.py
# @generated by refix codegen from toy.xml. Do not edit.
# fmt: off

from typing import TYPE_CHECKING

from refix import fields


class PtysSubGrp(fields.GroupInstance):
    party_sub_id = fields.Str(523)

    if TYPE_CHECKING:
        def __init__(self) -> None: ...
=== components/parties.py
# @generated by refix codegen from toy.xml. Do not edit.
# fmt: off

from typing import TYPE_CHECKING

from refix import fields

from .ptys_sub_grp import PtysSubGrp


class Parties(fields.GroupInstance):
    party_id = fields.Str(448)
    party_sub_ids = fields.Group(802, PtysSubGrp)

    if TYPE_CHECKING:
        def __init__(self) -> None: ...
=== messages/__init__.py
# @generated by refix codegen from toy.xml. Do not edit.
# fmt: off

from .new_order_single import NewOrderSingle as NewOrderSingle
=== messages/new_order_single.py
# @generated by refix codegen from toy.xml. Do not edit.
# fmt: off

from typing import TYPE_CHECKING, ClassVar

from refix import fields

from ..components.parties import Parties


class NewOrderSingle(fields.Message):
    """The NewOrderSingle (35=D) message."""

    MSG_TYPE: ClassVar[bytes] = b"D"

    party_ids = fields.Group(453, Parties)

    if TYPE_CHECKING:
        def __init__(self) -> None: ...
"#;
        assert_eq!(listing(&generated), expected);
        assert!(generated.warnings.is_empty());
    }

    #[test]
    fn a_shared_group_is_generated_once() {
        let code = code(
            vec![
                message("NewOrderSingle", "D", vec![component_ref("Parties")]),
                message("OrderCancelRequest", "F", vec![component_ref("Parties")]),
            ],
            party_fields(),
            party_components(),
        );

        let classes = code.matches("class Parties(fields.GroupInstance):");
        assert_eq!(classes.count(), 1);
        let declarations = code.matches("    party_ids = fields.Group(453, Parties)\n");
        assert_eq!(declarations.count(), 2);
    }

    #[test]
    fn a_message_group_nests_in_its_class() {
        let generated = generated(
            vec![message(
                "Logon",
                "A",
                vec![group(384, vec![field_ref(372)])],
            )],
            vec![
                field("NoMsgTypes", 384, DataType::NumInGroup, &[]),
                field("RefMsgType", 372, DataType::String, &[]),
            ],
            vec![],
        );

        assert!(listing(&generated).contains(
            r#"class Logon(fields.Message):
    """The Logon (35=A) message."""

    MSG_TYPE: ClassVar[bytes] = b"A"

    class MsgTypes(fields.GroupInstance):
        ref_msg_type = fields.Str(372)

        if TYPE_CHECKING:
            def __init__(self) -> None: ...

    msg_types = fields.Group(384, MsgTypes)

    if TYPE_CHECKING:
        def __init__(self) -> None: ...
"#
        ));
    }

    #[test]
    fn an_enum_named_like_a_message_is_renamed_with_a_warning() {
        let generated = generated(
            vec![message("SecurityStatus", "f", vec![field_ref(965)])],
            vec![field(
                "SecurityStatus",
                965,
                DataType::String,
                &[("1", "ACTIVE")],
            )],
            vec![],
        );

        assert!(listing(&generated).contains("class SecurityStatusEnum(enum.StrEnum):"));
        assert!(listing(&generated).contains("class SecurityStatus(fields.Message):"));
        assert!(
            listing(&generated)
                .contains("    security_status = fields.Str(965, SecurityStatusEnum)\n")
        );
        assert!(matches!(
            generated.warnings.as_slice(),
            [Warning::Renamed { .. }]
        ));
    }

    #[test]
    fn a_group_in_a_group_instance_gets_a_class_in_its_parents() {
        let generated = generated(
            vec![message(
                "NewOrderSingle",
                "D",
                vec![component_ref("Parties")],
            )],
            party_fields(),
            vec![component(
                "Parties",
                vec![group(
                    453,
                    vec![field_ref(448), group(802, vec![field_ref(523)])],
                )],
            )],
        );

        assert!(listing(&generated).contains(
            r#"class Parties(fields.GroupInstance):
    class PartySubIDs(fields.GroupInstance):
        party_sub_id = fields.Str(523)

        if TYPE_CHECKING:
            def __init__(self) -> None: ...

    party_id = fields.Str(448)
    party_sub_ids = fields.Group(802, PartySubIDs)

    if TYPE_CHECKING:
        def __init__(self) -> None: ...
"#
        ));
        assert_eq!(generated.warnings, vec![]);
    }
}
