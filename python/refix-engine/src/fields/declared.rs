use super::field::Field;
use super::group::Group;
use super::view::View;
use pyo3::types::{PyAnyMethods, PyTupleMethods, PyType, PyTypeMethods};
use pyo3::{Bound, PyResult, intern, pyclass};
use refix_message::{KnownTags, Tag};

// The tags an object of a class can contain, at any depth.
//
// Built on first use and kept on the class.
#[pyclass(frozen, name = "_DeclaredTags", module = "refix._core")]
pub(super) struct DeclaredTags {
    // The address of the class the tags were built for.
    // A subclass finds its parent's tags first and needs its own.
    class: usize,
    tags: Vec<Tag>,
}

impl DeclaredTags {
    // Returns the tags of `class`, building them on first use.
    pub(super) fn of<'py>(class: &Bound<'py, PyType>) -> PyResult<Bound<'py, Self>> {
        let py = class.py();
        let name = intern!(py, "_refix_tags");
        if let Some(found) = class.getattr_opt(name)?
            && let Ok(tags) = found.cast_into::<Self>()
            && tags.get().class == class.as_ptr() as usize
        {
            return Ok(tags);
        }

        let tags = Bound::new(py, Self::build(class)?)?;
        class.setattr(name, &tags)?;
        Ok(tags)
    }

    fn build(class: &Bound<'_, PyType>) -> PyResult<Self> {
        let mut tags = Vec::new();
        for field in declared(class)? {
            tags.push(Tag(field.get().tag()));

            // A group brings the tags of its instances.
            if let Ok(group) = field.cast::<Group>() {
                let instance = group.get().instance().bind(class.py());
                tags.extend_from_slice(&Self::of(instance)?.get().tags);
            }
        }
        tags.sort_unstable();
        tags.dedup();

        Ok(Self {
            class: class.as_ptr() as usize,
            tags,
        })
    }

    pub(super) fn known(&self) -> KnownTags<'_> {
        KnownTags::new(&self.tags)
    }
}

// Returns the fields `class` declares, maintaining the declaration order.
// A view stands for the fields of its component.
pub(super) fn declared<'py>(class: &Bound<'py, PyType>) -> PyResult<Vec<Bound<'py, Field>>> {
    let py = class.py();
    let mut fields = Vec::new();

    // Base classes come first, so their fields keep their place.
    for base in class.mro().iter().rev() {
        let namespace = base.getattr(intern!(py, "__dict__"))?;
        for value in namespace.call_method0(intern!(py, "values"))?.try_iter()? {
            let value = value?;
            if let Ok(field) = value.cast::<Field>() {
                fields.push(field.clone());
            } else if let Ok(view) = value.cast::<View>() {
                fields.extend(declared(view.get().component().bind(py))?);
            }
        }
    }

    Ok(fields)
}
