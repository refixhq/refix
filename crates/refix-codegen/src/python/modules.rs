use std::collections::{BTreeSet, HashMap};

use refix_dictionary::{MemberContext, dictionary};

use super::Error;
use super::names::Names;
use super::naming::module_name;
use crate::namespace::{Namespace, Owner};

/// The subpackage holding the module of every message or of every other
/// class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Subpackage {
    Components,
    Messages,
}

impl Subpackage {
    pub(super) fn name(self) -> &'static str {
        match self {
            Subpackage::Components => "components",
            Subpackage::Messages => "messages",
        }
    }
}

/// A class with a module of its own.
pub(super) struct Class {
    pub(super) name: String,
    pub(super) owner: Owner,
    pub(super) code: String,
    pub(super) uses: Uses,
}

/// The names from other modules that a class's body uses.
#[derive(Default)]
pub(super) struct Uses {
    pub(super) enums: BTreeSet<String>,
    pub(super) classes: BTreeSet<String>,
}

impl Uses {
    /// What the declarations of `members` name, and those of the groups
    /// declared among them, whose classes are in the same body.
    pub(super) fn of<'a>(
        members: impl Iterator<Item = dictionary::Member<'a>>,
        names: &Names,
    ) -> Self {
        let mut uses = Self::default();
        uses.collect(members, names);
        uses
    }

    fn collect<'a>(
        &mut self,
        members: impl Iterator<Item = dictionary::Member<'a>>,
        names: &Names,
    ) {
        for member in members {
            match member {
                dictionary::Member::Field { field, .. } => {
                    if let Some(class) = names.enum_class_of(field) {
                        self.enums.insert(class.to_owned());
                    }
                }
                dictionary::Member::Group(group) => {
                    if matches!(group.declared_in(), MemberContext::Component(_)) {
                        self.classes.insert(names.group_class(group).to_owned());
                    } else {
                        self.collect(group.members(), names);
                    }
                }
                dictionary::Member::Component(component) => {
                    self.classes.insert(component.name().to_owned());
                }
            }
        }
    }
}

/// The module of every class in a subpackage, by the class's name.
pub(super) struct Modules(HashMap<String, String>);

impl Modules {
    pub(super) fn new(classes: &[Class]) -> Result<Self, Error> {
        let mut taken = Namespace::default();
        let mut modules = HashMap::new();
        for class in classes {
            let module = module_name(&class.name);
            taken
                .claim(&module, class.owner.clone())
                .map_err(Error::ModuleClash)?;
            modules.insert(class.name.clone(), module);
        }
        Ok(Self(modules))
    }

    pub(super) fn of(&self, class: &str) -> &str {
        &self.0[class]
    }
}

/// The imports of a class's module in `subpackage`.
pub(super) fn emit_imports(class: &Class, subpackage: Subpackage, components: &Modules) -> String {
    let typing = match subpackage {
        Subpackage::Components => "from typing import TYPE_CHECKING\n",
        Subpackage::Messages => "from typing import TYPE_CHECKING, ClassVar\n",
    };
    let mut relative = Vec::new();
    let from_components = class.uses.classes.iter().map(|name| {
        let module = components.of(name);
        match subpackage {
            Subpackage::Components => (format!(".{module}"), name),
            Subpackage::Messages => (format!("..components.{module}"), name),
        }
    });
    let mut from_components: Vec<(String, &String)> = from_components.collect();
    from_components.sort();
    let enums: Vec<&str> = sorted_names(class.uses.enums.iter().map(String::as_str));
    let from_enums = (!enums.is_empty()).then(|| emit_from("..enums", &enums));
    let from_components = from_components
        .iter()
        .map(|(module, name)| emit_from(module, &[name.as_str()]));
    // A module further up comes first, as isort has it.
    match subpackage {
        Subpackage::Components => {
            relative.extend(from_enums);
            relative.extend(from_components);
        }
        Subpackage::Messages => {
            relative.extend(from_components);
            relative.extend(from_enums);
        }
    }
    let mut blocks = vec![typing.to_owned(), "from refix import fields\n".to_owned()];
    if !relative.is_empty() {
        blocks.push(relative.concat());
    }
    blocks.join("\n")
}

/// Class `names` in the order isort gives the names imported from one
/// module.
///
/// A name all in capitals reads as a constant to isort, which puts it first.
pub(super) fn sorted_names<'a>(names: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut names: Vec<&str> = names.collect();
    names.sort_by_key(|name| (!is_constant(name), name.to_lowercase(), *name));
    names
}

fn is_constant(name: &str) -> bool {
    name.len() > 1 && !name.chars().any(|c| c.is_lowercase())
}

/// The widest import isort leaves on one line by default.
const IMPORT_WIDTH: usize = 88;

/// An import of `names` from `module`, one per line when one line is too
/// long.
pub(super) fn emit_from(module: &str, names: &[&str]) -> String {
    let one_line = format!("from {module} import {}", names.join(", "));
    if one_line.len() <= IMPORT_WIDTH {
        return format!("{one_line}\n");
    }
    let lines: String = names.iter().map(|name| format!("    {name},\n")).collect();
    format!("from {module} import (\n{lines})\n")
}

#[cfg(test)]
mod tests {
    use refix_dictionary::quickfix;

    use super::{Class, Modules, Subpackage, Uses, emit_from, emit_imports, sorted_names};
    use crate::namespace::Owner;
    use crate::python::Error;
    use crate::python::names::Names;

    fn class(name: &str, enums: &[&str], classes: &[&str]) -> Class {
        Class {
            name: name.to_owned(),
            owner: Owner::Component(name.to_owned()),
            code: String::new(),
            uses: Uses {
                enums: enums.iter().map(|name| (*name).to_owned()).collect(),
                classes: classes.iter().map(|name| (*name).to_owned()).collect(),
            },
        }
    }

    #[test]
    fn uses_reach_into_a_group_the_class_declares_but_not_a_shared_one() {
        let xml = r#"<fix type="FIX" major="4" minor="4" servicepack="0">
  <header/>
  <messages>
    <message name="Logon" msgtype="A" msgcat="admin">
      <group name="NoMsgTypes" required="N">
        <field name="MsgDirection" required="N"/>
        <component name="Parties" required="N"/>
      </group>
    </message>
  </messages>
  <trailer/>
  <components>
    <component name="Parties">
      <group name="NoPartyIDs" required="N">
        <field name="PartyRole" required="N"/>
      </group>
    </component>
  </components>
  <fields>
    <field number="384" name="NoMsgTypes" type="NUMINGROUP"/>
    <field number="385" name="MsgDirection" type="CHAR">
      <value enum="S" description="SEND"/>
    </field>
    <field number="453" name="NoPartyIDs" type="NUMINGROUP"/>
    <field number="452" name="PartyRole" type="INT">
      <value enum="1" description="EXECUTING_FIRM"/>
    </field>
  </fields>
</fix>"#;
        let dictionary = quickfix::parse(xml).unwrap().dictionary;
        let names = Names::new(&dictionary, &mut Vec::new()).unwrap();
        let logon = dictionary.messages().next().unwrap();

        let uses = Uses::of(logon.members(), &names);

        assert_eq!(Vec::from_iter(&uses.enums), ["MsgDirection"]);
        assert_eq!(Vec::from_iter(&uses.classes), ["Parties"]);
    }

    #[test]
    fn a_message_module_imports_components_then_enums() {
        let components = [class("Parties", &[], &[]), class("Instrument", &[], &[])];
        let modules = Modules::new(&components).unwrap();
        let order = class(
            "NewOrderSingle",
            &["Side", "OrdType"],
            &["Parties", "Instrument"],
        );

        assert_eq!(
            emit_imports(&order, Subpackage::Messages, &modules),
            "from typing import TYPE_CHECKING, ClassVar\n\nfrom refix import fields\n\nfrom ..components.instrument import Instrument\nfrom ..components.parties import Parties\nfrom ..enums import OrdType, Side\n"
        );
    }

    #[test]
    fn a_component_module_imports_enums_then_its_neighbours() {
        let components = [
            class("PtysSubGrp", &[], &[]),
            class("Parties", &["PartyRole"], &["PtysSubGrp"]),
        ];
        let modules = Modules::new(&components).unwrap();

        assert_eq!(
            emit_imports(&components[1], Subpackage::Components, &modules),
            "from typing import TYPE_CHECKING\n\nfrom refix import fields\n\nfrom ..enums import PartyRole\nfrom .ptys_sub_grp import PtysSubGrp\n"
        );
    }

    #[test]
    fn a_class_that_names_nothing_imports_no_module_of_the_package() {
        let modules = Modules::new(&[]).unwrap();

        assert_eq!(
            emit_imports(&class("Logout", &[], &[]), Subpackage::Messages, &modules),
            "from typing import TYPE_CHECKING, ClassVar\n\nfrom refix import fields\n"
        );
    }

    #[test]
    fn two_classes_with_one_module_name_clash() {
        let classes = [class("SecurityID", &[], &[]), class("SecurityId", &[], &[])];

        let Err(Error::ModuleClash(clash)) = Modules::new(&classes) else {
            panic!("expected a module clash");
        };

        assert_eq!(clash.name, "security_id");
        assert_eq!(clash.first, Owner::Component("SecurityID".to_owned()));
        assert_eq!(clash.second, Owner::Component("SecurityId".to_owned()));
    }

    #[test]
    fn names_sort_without_regard_to_case() {
        assert_eq!(
            sorted_names(["URLLink", "Side", "UnitOfMeasure"].into_iter()),
            ["Side", "UnitOfMeasure", "URLLink"]
        );
    }

    #[test]
    fn a_name_all_in_capitals_sorts_first() {
        assert_eq!(
            sorted_names(["Advertisement", "IOI", "Email"].into_iter()),
            ["IOI", "Advertisement", "Email"]
        );
    }

    #[test]
    fn an_import_that_fits_stays_on_one_line() {
        assert_eq!(
            emit_from("..enums", &["OrdType", "Side"]),
            "from ..enums import OrdType, Side\n"
        );
    }

    #[test]
    fn a_long_import_puts_a_name_per_line() {
        let name = "UnderlyingStreamTerminationDateBusinessCenterGrp";

        assert_eq!(
            emit_from(
                ".underlying_stream_termination_date_business_center_grp",
                &[name]
            ),
            "from .underlying_stream_termination_date_business_center_grp import (\n    UnderlyingStreamTerminationDateBusinessCenterGrp,\n)\n"
        );
    }
}
