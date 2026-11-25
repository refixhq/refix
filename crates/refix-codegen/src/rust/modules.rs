use std::collections::{BTreeSet, HashMap};

use refix_dictionary::spec::Field;
use refix_dictionary::{Dictionary, dictionary};

use super::Error;
use super::groups::instance_groups;
use super::layout::{MAX_WIDTH, indent, list};
use super::messages::{message_groups, walks_groups};
use super::names::Names;
use super::naming::module_name;
use super::views::section_groups;
use crate::components::holds_group;
use crate::conversion::Conversion;
use crate::envelope::{Envelope, Section};
use crate::flat;
use crate::namespace::{Namespace, Owner};

/// The visibility of what a file shares with the rest of the generated
/// code and with nothing else.
///
/// Every such file is two modules below the root.
pub(super) const SHARED: &str = "pub(in super::super)";

/// The module holding the file of every message or of every other type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Folder {
    Components,
    Messages,
}

impl Folder {
    pub(super) fn name(self) -> &'static str {
        match self {
            Folder::Components => "components",
            Folder::Messages => "messages",
        }
    }
}

/// The file of one type, which also holds the types of the groups declared
/// in it.
pub(super) struct File {
    /// The type the file is named after.
    pub(super) name: String,
    pub(super) owner: Owner,
    pub(super) code: String,
    pub(super) uses: Uses,
}

/// What a file's code defines and what it names.
#[derive(Debug, Default, Eq, PartialEq)]
pub(super) struct Uses {
    /// The types the file defines, in order.
    pub(super) defined: Vec<String>,
    /// Names from `refix_message`.
    pub(super) runtime: BTreeSet<&'static str>,
    pub(super) enums: BTreeSet<String>,
    /// Every other generated type, the file's own included.
    pub(super) types: BTreeSet<String>,
}

impl Uses {
    pub(super) fn of_message(
        message: dictionary::Message<'_>,
        envelope: &Envelope,
        names: &Names,
    ) -> Self {
        let mut uses = Self::default();
        uses.defined.push(message.name().to_owned());
        uses.runtime.insert("RawMessage");
        if walks_groups(message, envelope) {
            uses.runtime.insert("LazyKnownTags");
        }
        uses.sections(&envelope.sections);
        uses.members(message.members(), names);
        for group in message_groups(message) {
            uses.instance(group, names);
        }
        uses
    }

    pub(super) fn of_component(component: dictionary::Component<'_>, names: &Names) -> Self {
        let mut uses = Self::default();
        uses.defined.push(component.name().to_owned());
        uses.runtime.insert("LazyKnownTags");
        if holds_group(component) {
            uses.runtime.insert("KnownTags");
        }
        uses.members(component.members(), names);
        uses
    }

    pub(super) fn of_view(
        section: Section,
        dictionary: &Dictionary,
        envelope: &Envelope,
        names: &Names,
    ) -> Self {
        let mut uses = Self::default();
        uses.defined.push(section.type_name().to_owned());
        uses.runtime.insert("LazyKnownTags");
        if envelope.with_groups.contains(&section) {
            uses.runtime.insert("KnownTags");
            uses.sections(&envelope.sections);
        }
        uses.members(section.listed(dictionary).into_iter(), names);
        for group in section_groups(section, dictionary) {
            uses.instance(group, names);
        }
        uses
    }

    pub(super) fn of_group(group: dictionary::Group<'_>, names: &Names) -> Self {
        let mut uses = Self::default();
        uses.instance(group, names);
        uses
    }

    /// The type of a group's instances, then those of the groups declared
    /// in it.
    fn instance(&mut self, group: dictionary::Group<'_>, names: &Names) {
        self.defined.push(names.group_type(group).to_owned());
        self.runtime.extend(["GroupTable", "LazyKnownTags", "Tag"]);
        // The table names the table of every group in an instance.
        for member in flat::members(group.members()) {
            if let flat::Member::Group(nested) = member {
                self.types.insert(names.group_type(nested).to_owned());
            }
        }
        self.members(group.members(), names);
        for nested in instance_groups(group) {
            self.instance(nested, names);
        }
    }

    fn sections(&mut self, sections: &[Section]) {
        let types = sections.iter().map(|section| section.type_name());
        self.types.extend(types.map(str::to_owned));
    }

    /// What the accessors of `members` name, and the known tags that list
    /// them.
    fn members<'a>(
        &mut self,
        members: impl Iterator<Item = dictionary::Member<'a>>,
        names: &Names,
    ) {
        for member in members {
            match member {
                dictionary::Member::Field { field, .. } => self.field(field, names),
                dictionary::Member::Group(group) => {
                    self.runtime.extend(["Instances", "InvalidValue", "Tag"]);
                    self.types.insert(names.group_type(group).to_owned());
                }
                dictionary::Member::Component(component) => {
                    self.types.insert(component.name().to_owned());
                }
            }
        }
    }

    fn field(&mut self, field: &Field, names: &Names) {
        let conversion = Conversion::of(field);
        self.runtime.insert("Tag");
        if !matches!(conversion, Conversion::Bytes | Conversion::Raw) {
            self.runtime.insert("InvalidValue");
        }
        self.runtime.extend(match conversion {
            Conversion::Decimal => Some("Decimal"),
            Conversion::Date => Some("Date"),
            Conversion::MonthYear => Some("MonthYear"),
            Conversion::Time => Some("Time"),
            Conversion::UtcTimestamp => Some("UtcTimestamp"),
            Conversion::MultipleValues | Conversion::MultipleStr => Some("MultipleValues"),
            _ => None,
        });
        if conversion.is_enum() {
            self.enums.insert(names.enum_type(field).to_owned());
        }
    }
}

/// The module of every file in a folder, by the name of the file's type.
pub(super) struct Modules(HashMap<String, String>);

impl Modules {
    pub(super) fn new(files: &[File]) -> Result<Self, Error> {
        let mut taken = Namespace::default();
        let mut modules = HashMap::new();
        for file in files {
            let module = module_name(&file.name);
            taken
                .claim(&module, file.owner.clone())
                .map_err(Error::ModuleClash)?;
            modules.insert(file.name.clone(), module);
        }
        Ok(Self(modules))
    }

    pub(super) fn of(&self, name: &str) -> &str {
        &self.0[name]
    }
}

/// The imports of a file in `folder`.
pub(super) fn emit_imports(uses: &Uses, folder: Folder) -> String {
    let runtime: Vec<&str> = uses.runtime.iter().copied().collect();
    let enums: Vec<&str> = uses.enums.iter().map(String::as_str).collect();
    let types = uses.types.iter().map(String::as_str);
    let types: Vec<&str> = types
        .filter(|name| !uses.defined.iter().any(|defined| defined == name))
        .collect();
    let components = match folder {
        Folder::Components => "super",
        Folder::Messages => "super::super::components",
    };
    let generated = match folder {
        Folder::Components => [("super::super::enums", enums), (components, types)],
        Folder::Messages => [(components, types), ("super::super::enums", enums)],
    };
    let generated: String = generated
        .iter()
        .map(|(module, names)| emit_use("use", module, names))
        .collect();
    [emit_use("use", "refix_message", &runtime), generated]
        .into_iter()
        .filter(|block| !block.is_empty())
        .collect::<Vec<String>>()
        .join("\n")
}

/// A folder's root file, which declares the module of every file and offers
/// the types of each.
pub(super) fn emit_folder(files: &[File], modules: &Modules) -> String {
    let mut entries: Vec<(&str, Vec<&str>)> = files
        .iter()
        .map(|file| {
            let mut types: Vec<&str> = file.uses.defined.iter().map(String::as_str).collect();
            types.sort_unstable();
            (modules.of(&file.name), types)
        })
        .collect();
    entries.sort_unstable();
    let declarations: String = entries
        .iter()
        .map(|(module, _)| format!("mod {module};\n"))
        .collect();
    let offered: String = entries
        .iter()
        .map(|(module, types)| indent(&emit_path(module, types, ",", MAX_WIDTH - 4), 1))
        .collect();
    format!("{declarations}\n{UNUSED}pub use self::{{\n{offered}}};\n")
}

/// Keeps a crate that does not name every type free of warnings.
pub(super) const UNUSED: &str = "#[allow(unused_imports)]\n";

/// A `keyword` statement importing `names` from `module`, or nothing
/// without names.
pub(super) fn emit_use(keyword: &str, module: &str, names: &[&str]) -> String {
    if names.is_empty() {
        return String::new();
    }
    emit_path(&format!("{keyword} {module}"), names, ";", MAX_WIDTH)
}

/// The path of `names` in `module`, in braces when there are several.
fn emit_path(module: &str, names: &[&str], tail: &str, width: usize) -> String {
    match names {
        [name] => format!("{module}::{name}{tail}\n"),
        _ => {
            let names: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
            list(
                &format!("{module}::{{"),
                &names,
                &format!("}}{tail}"),
                width,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use refix_dictionary::quickfix;

    use super::{File, Folder, Modules, Uses, emit_folder, emit_imports};
    use crate::components::components;
    use crate::envelope::{Envelope, Section};
    use crate::namespace::Owner;
    use crate::rust::Error;
    use crate::rust::names::Names;

    const XML: &str = r#"<fix type="FIX" major="4" minor="4" servicepack="0">
  <header>
    <field name="MsgType" required="Y"/>
    <group name="NoHops" required="N">
      <field name="HopCompID" required="N"/>
    </group>
  </header>
  <messages>
    <message name="Logon" msgtype="A" msgcat="admin">
      <group name="NoMsgTypes" required="N">
        <field name="MsgDirection" required="N"/>
        <component name="Parties" required="N"/>
      </group>
    </message>
    <message name="Logout" msgtype="5" msgcat="admin">
      <component name="Instrument" required="N"/>
    </message>
  </messages>
  <trailer>
    <field name="CheckSum" required="Y"/>
  </trailer>
  <components>
    <component name="Parties">
      <group name="NoPartyIDs" required="N">
        <field name="PartyRole" required="N"/>
      </group>
    </component>
    <component name="Instrument">
      <field name="Symbol" required="N"/>
      <component name="Parties" required="N"/>
    </component>
  </components>
  <fields>
    <field number="35" name="MsgType" type="STRING"/>
    <field number="627" name="NoHops" type="NUMINGROUP"/>
    <field number="628" name="HopCompID" type="STRING"/>
    <field number="10" name="CheckSum" type="STRING"/>
    <field number="384" name="NoMsgTypes" type="NUMINGROUP"/>
    <field number="385" name="MsgDirection" type="CHAR">
      <value enum="S" description="SEND"/>
    </field>
    <field number="453" name="NoPartyIDs" type="NUMINGROUP"/>
    <field number="452" name="PartyRole" type="INT">
      <value enum="1" description="EXECUTING_FIRM"/>
    </field>
    <field number="55" name="Symbol" type="STRING"/>
  </fields>
</fix>"#;

    fn uses(defined: &[&str], runtime: &[&'static str], enums: &[&str], types: &[&str]) -> Uses {
        let owned = |names: &[&str]| -> Vec<String> {
            names.iter().map(|name| (*name).to_owned()).collect()
        };
        Uses {
            defined: owned(defined),
            runtime: runtime.iter().copied().collect(),
            enums: owned(enums).into_iter().collect(),
            types: owned(types).into_iter().collect(),
        }
    }

    fn file(name: &str, uses: Uses) -> File {
        File {
            name: name.to_owned(),
            owner: Owner::Component(name.to_owned()),
            code: String::new(),
            uses,
        }
    }

    #[test]
    fn a_message_file_defines_its_groups_and_names_what_they_read() {
        let dictionary = quickfix::parse(XML).unwrap().dictionary;
        let names = Names::new(&dictionary, &mut Vec::new()).unwrap();
        let logon = dictionary.messages().next().unwrap();

        assert_eq!(
            Uses::of_message(logon, &Envelope::of(&dictionary), &names),
            uses(
                &["Logon", "LogonMsgTypes"],
                &[
                    "GroupTable",
                    "Instances",
                    "InvalidValue",
                    "LazyKnownTags",
                    "RawMessage",
                    "Tag"
                ],
                &["MsgDirection"],
                &["Header", "LogonMsgTypes", "Parties", "Trailer"],
            )
        );
    }

    #[test]
    fn a_component_holding_a_group_keeps_the_known_tags() {
        let dictionary = quickfix::parse(XML).unwrap().dictionary;
        let names = Names::new(&dictionary, &mut Vec::new()).unwrap();
        let instrument = components(&dictionary)[0];

        assert_eq!(
            Uses::of_component(instrument, &names),
            uses(
                &["Instrument"],
                &[
                    "Instances",
                    "InvalidValue",
                    "KnownTags",
                    "LazyKnownTags",
                    "Tag"
                ],
                &[],
                &["Parties"],
            )
        );
    }

    #[test]
    fn a_view_that_walks_groups_names_every_section() {
        let dictionary = quickfix::parse(XML).unwrap().dictionary;
        let names = Names::new(&dictionary, &mut Vec::new()).unwrap();
        let envelope = Envelope::of(&dictionary);

        assert_eq!(
            Uses::of_view(Section::Header, &dictionary, &envelope, &names),
            uses(
                &["Header", "HeaderHops"],
                &[
                    "GroupTable",
                    "Instances",
                    "InvalidValue",
                    "KnownTags",
                    "LazyKnownTags",
                    "Tag"
                ],
                &[],
                &["Header", "HeaderHops", "Trailer"],
            )
        );
        assert_eq!(
            Uses::of_view(Section::Trailer, &dictionary, &envelope, &names),
            uses(
                &["Trailer"],
                &["InvalidValue", "LazyKnownTags", "Tag"],
                &[],
                &[]
            )
        );
    }

    #[test]
    fn a_shared_group_file_names_its_enums() {
        let dictionary = quickfix::parse(XML).unwrap().dictionary;
        let names = Names::new(&dictionary, &mut Vec::new()).unwrap();
        let parties = crate::rust::groups::shared_groups(&dictionary)[0];

        assert_eq!(
            Uses::of_group(parties, &names),
            uses(
                &["Parties"],
                &["GroupTable", "InvalidValue", "LazyKnownTags", "Tag"],
                &["PartyRole"],
                &[],
            )
        );
    }

    #[test]
    fn a_message_file_imports_components_then_enums() {
        let order = uses(
            &["NewOrderSingle", "NewOrderSingleAllocs"],
            &["RawMessage", "Tag"],
            &["Side", "OrdType"],
            &["Parties", "NewOrderSingleAllocs", "Header"],
        );

        assert_eq!(
            emit_imports(&order, Folder::Messages),
            "use refix_message::{RawMessage, Tag};\n\nuse super::super::components::{Header, Parties};\nuse super::super::enums::{OrdType, Side};\n"
        );
    }

    #[test]
    fn a_component_file_imports_enums_then_its_neighbours() {
        let parties = uses(
            &["Parties"],
            &["Tag"],
            &["PartyRole"],
            &["Parties", "PtysSubGrp"],
        );

        assert_eq!(
            emit_imports(&parties, Folder::Components),
            "use refix_message::Tag;\n\nuse super::super::enums::PartyRole;\nuse super::PtysSubGrp;\n"
        );
    }

    #[test]
    fn a_file_that_names_no_generated_type_imports_the_runtime_alone() {
        let logout = uses(&["Logout"], &["RawMessage"], &[], &[]);

        assert_eq!(
            emit_imports(&logout, Folder::Messages),
            "use refix_message::RawMessage;\n"
        );
    }

    #[test]
    fn a_long_import_fills_indented_lines() {
        let runtime = [
            "Date",
            "Decimal",
            "Instances",
            "InvalidValue",
            "LazyKnownTags",
            "MonthYear",
            "MultipleValues",
            "RawMessage",
            "Tag",
            "Time",
        ];

        assert_eq!(
            emit_imports(&uses(&[], &runtime, &[], &[]), Folder::Messages),
            "use refix_message::{\n    Date, Decimal, Instances, InvalidValue, LazyKnownTags, MonthYear, MultipleValues, RawMessage,\n    Tag, Time,\n};\n"
        );
    }

    #[test]
    fn a_folder_declares_each_module_and_offers_its_types() {
        let files = [
            file("Parties", uses(&["Parties"], &[], &[], &[])),
            file("Header", uses(&["Header", "HeaderHops"], &[], &[], &[])),
        ];
        let modules = Modules::new(&files).unwrap();

        assert_eq!(
            emit_folder(&files, &modules),
            "mod header;\nmod parties;\n\n#[allow(unused_imports)]\npub use self::{\n    header::{Header, HeaderHops},\n    parties::Parties,\n};\n"
        );
    }

    #[test]
    fn two_files_with_one_module_name_clash() {
        let files = [
            file("SecurityID", Uses::default()),
            file("SecurityId", Uses::default()),
        ];

        let Err(Error::ModuleClash(clash)) = Modules::new(&files) else {
            panic!("expected a module clash");
        };

        assert_eq!(clash.name, "security_id");
        assert_eq!(clash.first, Owner::Component("SecurityID".to_owned()));
        assert_eq!(clash.second, Owner::Component("SecurityId".to_owned()));
    }
}
