use refix_dictionary::dictionary;

use super::docs::code;
use super::groups::{KNOWN_TAGS, emit_known_tags};
use super::layout::indent;
use super::messages::{Lifetime, emit_accessors};
use super::names::Names;
use super::printing::emit_debug;
use crate::components::holds_group;

/// A component's view over the scope of whatever includes the component.
///
/// A view whose component holds a group keeps the known tags to walk it
/// with, which its owner passes. `Debug` prints the component's own fields.
pub(super) fn emit_component(component: dictionary::Component<'_>, names: &Names) -> String {
    let name = component.name();
    let fields = if holds_group(component) {
        "(refix_message::Scope<'a>, KnownTags<'static>)"
    } else {
        "(refix_message::Scope<'a>)"
    };
    let tags = emit_known_tags(component.members(), &[], names);

    let mut methods = vec![
        indent(&tags, 1),
        "    pub fn raw(&self) -> refix_message::Scope<'a> {\n        self.0\n    }\n".to_owned(),
    ];
    methods.extend(emit_accessors(
        component.members(),
        names,
        Lifetime::Scope,
        "self.1",
    ));

    [
        format!(
            "/// The fields of the {} component.\n#[derive(Clone, Copy)]\npub struct {name}<'a>{fields};\n",
            code(name)
        ),
        format!("impl<'a> {name}<'a> {{\n{}}}\n", methods.join("\n")),
        emit_debug(name, &format!("self.0.display_known(&{KNOWN_TAGS})")),
    ]
    .join("\n")
}

/// The method returning a component's view.
///
/// It sits on whatever includes the component. A view that walks groups
/// gets the `known_tags` of the scope.
pub(super) fn emit_component_accessor(
    component: dictionary::Component<'_>,
    names: &Names,
    lifetime: Lifetime,
    known_tags: &str,
) -> String {
    let name = names.component_accessor(component);
    let type_name = component.name();
    let (scope, lifetime) = match lifetime {
        Lifetime::Receiver => ("self.0.scope()", "'_"),
        Lifetime::Scope => ("self.0", "'a"),
    };
    let arguments = if holds_group(component) {
        format!("{scope}, {known_tags}")
    } else {
        scope.to_owned()
    };
    format!(
        "    /// The {} component.\n    pub fn {name}(&self) -> {type_name}<{lifetime}> {{\n        {type_name}({arguments})\n    }}\n",
        code(type_name)
    )
}

#[cfg(test)]
mod tests {
    use super::{emit_component, emit_component_accessor};
    use crate::components::components;
    use crate::rust::groups::KNOWN_TAGS;
    use crate::rust::messages::Lifetime;
    use crate::rust::names::Names;
    use crate::test_utils::instrument_dictionary;

    #[test]
    fn a_view_reads_its_own_members_and_prints_its_own_fields() {
        let dictionary = instrument_dictionary();
        let names = Names::new(&dictionary, &mut Vec::new()).unwrap();
        let instrument = components(&dictionary)[0];

        assert_eq!(
            emit_component(instrument, &names),
            r#"/// The fields of the `Instrument` component.
#[derive(Clone, Copy)]
pub struct Instrument<'a>(refix_message::Scope<'a>, KnownTags<'static>);

impl<'a> Instrument<'a> {
    const fn known_tags() -> &'static LazyKnownTags {
        static TAGS: LazyKnownTags = LazyKnownTags::new(
            &[Tag(55), Tag(454)],
            &[SecAltIDGrp::known_tags(), SecurityXML::known_tags()],
        );
        &TAGS
    }

    pub fn raw(&self) -> refix_message::Scope<'a> {
        self.0
    }

    /// The `Symbol` (55) field.
    #[doc(alias = "Symbol")]
    pub fn symbol(&self) -> Result<Option<&'a str>, InvalidValue> {
        self.0.get_str(Tag(55))
    }

    /// The instances of the `NoSecurityAltID` (454) group.
    #[doc(alias = "NoSecurityAltID")]
    pub fn security_alt_id(&self) -> Result<Instances<'a, SecAltIDGrp<'a>>, InvalidValue> {
        self.0.get_group(&SecAltIDGrp::TABLE, &self.1).map(Instances::from)
    }

    /// The `SecurityXML` component.
    pub fn security_xml(&self) -> SecurityXML<'a> {
        SecurityXML(self.0)
    }
}

impl std::fmt::Debug for Instrument<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Instrument({})", self.0.display_known(&Self::known_tags().get()))
    }
}
"#
        );
    }

    #[test]
    fn a_view_without_a_group_keeps_no_known_tags() {
        let dictionary = instrument_dictionary();
        let names = Names::new(&dictionary, &mut Vec::new()).unwrap();
        let order_qty_data = components(&dictionary)[2];

        assert!(
            emit_component(order_qty_data, &names)
                .contains("pub struct OrderQtyData<'a>(refix_message::Scope<'a>);\n")
        );
    }

    #[test]
    fn a_message_passes_its_scope_and_known_tags() {
        let dictionary = instrument_dictionary();
        let names = Names::new(&dictionary, &mut Vec::new()).unwrap();
        let instrument = components(&dictionary)[0];

        assert_eq!(
            emit_component_accessor(instrument, &names, Lifetime::Receiver, KNOWN_TAGS),
            "    /// The `Instrument` component.\n    pub fn instrument(&self) -> Instrument<'_> {\n        Instrument(self.0.scope(), Self::known_tags().get())\n    }\n"
        );
    }

    #[test]
    fn a_scope_passes_itself_and_its_known_tags() {
        let dictionary = instrument_dictionary();
        let names = Names::new(&dictionary, &mut Vec::new()).unwrap();
        let instrument = components(&dictionary)[0];

        assert_eq!(
            emit_component_accessor(instrument, &names, Lifetime::Scope, "self.1"),
            "    /// The `Instrument` component.\n    pub fn instrument(&self) -> Instrument<'a> {\n        Instrument(self.0, self.1)\n    }\n"
        );
    }

    #[test]
    fn a_view_without_a_group_takes_the_scope_alone() {
        let dictionary = instrument_dictionary();
        let names = Names::new(&dictionary, &mut Vec::new()).unwrap();
        let order_qty_data = components(&dictionary)[2];

        assert_eq!(
            emit_component_accessor(order_qty_data, &names, Lifetime::Receiver, KNOWN_TAGS),
            "    /// The `OrderQtyData` component.\n    pub fn order_qty_data(&self) -> OrderQtyData<'_> {\n        OrderQtyData(self.0.scope())\n    }\n"
        );
    }
}
