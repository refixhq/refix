/// An int-based value as a canonical literal, leading zeros dropped.
///
/// The resolver guarantees an optional `-` followed by digits.
pub(crate) fn int_literal(value: &str) -> String {
    let (sign, digits) = match value.strip_prefix('-') {
        Some(digits) => ("-", digits),
        None => ("", value),
    };
    match digits.trim_start_matches('0') {
        "" => "0".to_owned(),
        digits => format!("{sign}{digits}"),
    }
}
