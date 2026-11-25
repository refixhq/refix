/// A `Debug` that prints the name of a view or group type around `shown`.
pub(super) fn emit_debug(name: &str, shown: &str) -> String {
    format!(
        "impl std::fmt::Debug for {name}<'_> {{\n    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{\n        write!(f, \"{name}({{}})\", {shown})\n    }}\n}}\n"
    )
}
