use super::field::Field;
use pyo3::types::{PyAnyMethods, PyTupleMethods, PyType, PyTypeMethods};
use pyo3::{Bound, PyResult, intern};
use refix_message::{KnownTags, Tag};

// The tags an object of a class can contain.
pub(super) struct DeclaredTags {
    tags: Vec<Tag>,
}

impl DeclaredTags {
    pub(super) fn of(class: &Bound<'_, PyType>) -> PyResult<Self> {
        let mut tags = Vec::new();
        for field in declared(class)? {
            tags.push(Tag(field.get().tag()));
        }
        tags.sort_unstable();
        tags.dedup();

        Ok(Self { tags })
    }

    pub(super) fn known(&self) -> KnownTags<'_> {
        KnownTags::new(&self.tags)
    }
}

// Returns the fields `class` declares, maintaining the declaration order.
pub(super) fn declared<'py>(class: &Bound<'py, PyType>) -> PyResult<Vec<Bound<'py, Field>>> {
    let py = class.py();
    let mut fields = Vec::new();

    // Base classes come first, so their fields keep their place.
    for base in class.mro().iter().rev() {
        let namespace = base.getattr(intern!(py, "__dict__"))?;
        for value in namespace.call_method0(intern!(py, "values"))?.try_iter()? {
            if let Ok(field) = value?.cast_into::<Field>() {
                fields.push(field);
            }
        }
    }

    Ok(fields)
}
