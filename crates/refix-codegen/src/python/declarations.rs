use refix_dictionary::dictionary;
use refix_dictionary::spec::Field;

use super::names::Names;
use super::naming::nanos_declaration;
use crate::conversion::Conversion;

/// The declarations of a message, component or group instance.
///
/// The first one in a group instance is the field its instances start with.
/// A component with fields is read through a view.
pub(super) fn emit_declarations<'a>(
    members: impl Iterator<Item = dictionary::Member<'a>>,
    names: &Names,
) -> String {
    members
        .map(|member| match member {
            dictionary::Member::Field { field, .. } => emit_field(field, names),
            dictionary::Member::Group(group) => emit_group(group, names),
            dictionary::Member::Component(component) => emit_view(component, names),
        })
        .collect()
}

/// The declaration reading a component as a view.
fn emit_view(component: dictionary::Component<'_>, names: &Names) -> String {
    let name = names.component_property(component);
    format!("    {name} = fields.View({})\n", component.name())
}

/// The declaration reading `field`, followed by the one reading its
/// nanoseconds where Python holds the value only to the microsecond.
fn emit_field(field: &Field, names: &Names) -> String {
    let name = names.property(field);
    let tag = field.tag;
    let declaration = match Conversion::of(field) {
        Conversion::Str => format!("fields.Str({tag})"),
        Conversion::Int => format!("fields.Int({tag})"),
        Conversion::Bool => format!("fields.Bool({tag})"),
        Conversion::Decimal => format!("fields.Decimal({tag})"),
        Conversion::Date => format!("fields.Date({tag})"),
        Conversion::MonthYear => format!("fields.MonthYear({tag})"),
        Conversion::Time => format!("fields.Time({tag})"),
        Conversion::UtcTimestamp => format!("fields.UtcTimestamp({tag})"),
        Conversion::StrEnum => format!("fields.Str({tag}, {})", names.enum_class(field)),
        Conversion::IntEnum => format!("fields.Int({tag}, {})", names.enum_class(field)),
        Conversion::MultipleValues => {
            format!("fields.MultipleValues({tag}, {})", names.enum_class(field))
        }
        Conversion::MultipleStr => format!("fields.MultipleValues({tag})"),
        Conversion::Bytes | Conversion::Raw => format!("fields.Bytes({tag})"),
    };
    let line = format!("    {name} = {declaration}\n");
    match nanos_declaration(field) {
        Some(class) => {
            let nanos = names.nanos_property(field);
            format!("{line}    {nanos} = fields.{class}({tag})\n")
        }
        None => line,
    }
}

/// The declaration reading the instances of `group`.
///
/// It names the group's class without a path, as the class is in the module
/// or in the body the declaration is in.
fn emit_group(group: dictionary::Group<'_>, names: &Names) -> String {
    let name = names.group_property(group);
    let count_tag = group.count_field().tag;
    let class = names.group_class(group);
    format!("    {name} = fields.Group({count_tag}, {class}.Instance)\n")
}
