//! The check that the installed `refix` is new enough for the module.

/// The oldest `refix` that generated modules run on, as major and minor.
pub(super) const MINIMUM: (u32, u32) = (0, 8);

/// The lines that refuse an older `refix` with an error naming both versions.
///
/// They read only `refix.__version__`, which every release has.
pub(super) fn emit_runtime_check() -> String {
    let (major, minor) = MINIMUM;
    format!(
        r#"if tuple(map(int, refix.__version__.split(".")[:2])) < ({major}, {minor}):
    raise ImportError(
        f"{{__name__}} needs refix {major}.{minor} or newer, but refix {{refix.__version__}} is installed"
    )
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const STOCK_MANIFESTS: [&str; 2] = [
        include_str!("../../../../python/refix-fix42/pyproject.toml"),
        include_str!("../../../../python/refix-fix44/pyproject.toml"),
    ];

    #[test]
    fn the_check_compares_the_major_and_minor_as_numbers() {
        let check = emit_runtime_check();

        assert!(check.starts_with(
            "if tuple(map(int, refix.__version__.split(\".\")[:2])) < (0, 8):\n    raise ImportError(\n"
        ));
    }

    #[test]
    fn the_error_names_the_module_and_both_versions() {
        let check = emit_runtime_check();

        assert!(check.contains(
            "f\"{__name__} needs refix 0.8 or newer, but refix {refix.__version__} is installed\"\n"
        ));
    }

    #[test]
    fn the_stock_packages_depend_on_the_minimum() {
        let (major, minor) = MINIMUM;
        let dependency = format!("dependencies = [\"refix>={major}.{minor}\"]\n");

        for manifest in STOCK_MANIFESTS {
            assert!(manifest.contains(&dependency));
        }
    }
}
