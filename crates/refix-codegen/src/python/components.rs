use refix_dictionary::dictionary;

use super::declarations::emit_declarations;
use super::names::Names;

/// A component's class, read as a view of whatever includes the component.
pub(super) fn emit_component(component: dictionary::Component<'_>, names: &Names) -> String {
    format!(
        "class {}(fields.Component):\n{}",
        component.name(),
        emit_declarations(component.members(), names)
    )
}

#[cfg(test)]
mod tests {
    use super::emit_component;
    use crate::components::components;
    use crate::python::names::Names;
    use crate::test_utils::instrument_dictionary;

    #[test]
    fn a_component_class_declares_its_own_members() {
        let dictionary = instrument_dictionary();
        let names = Names::new(&dictionary, &mut Vec::new()).unwrap();
        let instrument = components(&dictionary)[0];

        assert_eq!(
            emit_component(instrument, &names),
            r#"class Instrument(fields.Component):
    symbol = fields.Str(55)
    security_alt_id = fields.Group(454, SecAltIDGrp)
    security_xml = fields.View(SecurityXML)
"#
        );
    }
}
