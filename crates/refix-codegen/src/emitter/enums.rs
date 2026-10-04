use refix_dictionary::Field;

use super::{Error, naming::variant_name};

pub(super) fn emit_enum(field: &Field) -> Result<String, Error> {
    let name = &field.name;
    let variants = field
        .values
        .iter()
        .map(|value| variant_name(field, value))
        .collect::<Result<Vec<_>, _>>()?;

    let declarations: String = variants
        .iter()
        .map(|variant| format!("    {variant},\n"))
        .collect();
    let arms: String = field
        .values
        .iter()
        .zip(&variants)
        .map(|(value, variant)| format!("            b\"{}\" => Self::{variant},\n", value.value))
        .collect();

    let declaration = format!(
        "#[derive(Clone, Copy, Debug, Eq, PartialEq)]\npub enum {name}<'a> {{\n{declarations}    Unrecognized(&'a [u8]),\n}}\n"
    );
    let implementation = format!(
        "impl<'a> {name}<'a> {{\n    pub fn from_bytes(bytes: &'a [u8]) -> Self {{\n        match bytes {{\n{arms}            unrecognized => Self::Unrecognized(unrecognized),\n        }}\n    }}\n}}\n"
    );

    Ok(format!("{declaration}\n{implementation}"))
}
