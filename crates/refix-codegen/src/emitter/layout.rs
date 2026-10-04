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

/// A slice literal of `items` between `head` and `tail`.
///
/// It stays on one line when that fits in `width`. Otherwise the items fill
/// indented lines.
pub(super) fn slice(head: &str, items: &[String], tail: &str, width: usize) -> String {
    let one_line = format!("{head}&[{}]{tail}", items.join(", "));
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
    format!("{head}&[\n{lines}]{tail}\n")
}

#[cfg(test)]
mod tests {
    use super::{indent, slice};

    fn tags(tags: &[u32]) -> Vec<String> {
        tags.iter().map(|tag| format!("Tag({tag})")).collect()
    }

    #[test]
    fn indents_every_line_but_blank_ones() {
        assert_eq!(indent("a\n\nb\n", 2), "        a\n\n        b\n");
    }

    #[test]
    fn a_slice_that_fits_stays_on_one_line() {
        assert_eq!(
            slice("f(", &tags(&[1, 2]), ");", 21),
            "f(&[Tag(1), Tag(2)]);\n"
        );
    }

    #[test]
    fn a_long_slice_fills_indented_lines() {
        assert_eq!(
            slice("f(", &tags(&[1, 2, 3, 4, 5]), ");", 20),
            "f(&[\n    Tag(1), Tag(2),\n    Tag(3), Tag(4),\n    Tag(5),\n]);\n"
        );
    }

    #[test]
    fn an_empty_slice_stays_on_one_line() {
        assert_eq!(slice("f(", &[], ");", 1), "f(&[]);\n");
    }
}
