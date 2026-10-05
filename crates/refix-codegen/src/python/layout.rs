/// The widest line the emitter aims for.
pub(super) const MAX_WIDTH: usize = 100;

/// Indents every non-empty line of `text` by `levels` levels.
pub(super) fn indent(text: &str, levels: usize) -> String {
    let pad = "    ".repeat(levels);
    text.lines()
        .map(|line| {
            if line.is_empty() {
                "\n".to_owned()
            } else {
                format!("{pad}{line}\n")
            }
        })
        .collect()
}

/// A one-line tuple literal of `items`.
pub(super) fn tuple(items: &[String]) -> String {
    match items {
        [] => "()".to_owned(),
        [item] => format!("({item},)"),
        items => format!("({})", items.join(", ")),
    }
}

/// A tuple literal of `items` between `head` and `tail`.
///
/// It stays on one line when that fits in `width`. Otherwise the items fill
/// indented lines.
pub(super) fn tuple_lines(head: &str, items: &[String], tail: &str, width: usize) -> String {
    let one_line = format!("{head}{}{tail}", tuple(items));
    if one_line.len() <= width || items.is_empty() {
        return format!("{one_line}\n");
    }

    let mut lines: Vec<String> = Vec::new();
    for item in items {
        let item = format!("{item},");
        match lines.last_mut() {
            Some(line) if line.len() + 1 + item.len() <= width - 4 => {
                line.push(' ');
                line.push_str(&item);
            }
            _ => lines.push(item),
        }
    }
    let lines = indent(&(lines.join("\n") + "\n"), 1);
    format!("{head}(\n{lines}){tail}\n")
}

#[cfg(test)]
mod tests {
    use super::{tuple, tuple_lines};

    fn items(values: &[u32]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn a_tuple_of_one_keeps_its_comma() {
        assert_eq!(tuple(&items(&[])), "()");
        assert_eq!(tuple(&items(&[523])), "(523,)");
        assert_eq!(tuple(&items(&[448, 452])), "(448, 452)");
    }

    #[test]
    fn a_long_tuple_fills_indented_lines() {
        assert_eq!(
            tuple_lines("f(", &items(&[1, 2, 3, 4, 5, 6]), ")", 12),
            "f((\n    1, 2, 3,\n    4, 5, 6,\n))\n"
        );
    }
}
