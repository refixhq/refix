/// The constructor a type checker sees on a generated class.
///
/// It takes no arguments. A received message is wrapped with `from_raw`.
pub(super) fn emit_constructor() -> String {
    "    if TYPE_CHECKING:\n        def __init__(self) -> None: ...\n".to_owned()
}
