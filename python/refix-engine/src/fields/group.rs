use super::GroupInstance;
use super::declared::{DeclaredTags, declared};
use super::field::Field;
use crate::raw;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::types::{PyAnyMethods, PyTuple, PyType, PyTypeMethods};
use pyo3::{Bound, Py, PyAny, PyClassInitializer, PyResult, pyclass, pymethods};
use refix_message::{OwnedGroupTable, Tag};

/// A repeating group read as a tuple of its instances.
#[pyclass(extends = Field, frozen, module = "refix.fields")]
pub(crate) struct Group {
    instance: Py<PyType>,
}

impl Group {
    // Returns the table of the group, built from the fields of the instance class.
    fn table(slf: &Bound<'_, Self>) -> PyResult<OwnedGroupTable> {
        let class = slf.get().instance.bind(slf.py());
        let fields = declared(class)?;
        let Some(first) = fields.first() else {
            let name = class.qualname()?;
            return Err(PyTypeError::new_err(format!("'{name}' declares no fields")));
        };

        let count_tag = Tag(slf.as_super().get().tag());
        let delimiter = Tag(first.get().tag());
        let mut members: Vec<_> = fields.iter().map(|field| Tag(field.get().tag())).collect();
        members.sort_unstable();
        members.dedup();

        OwnedGroupTable::new(count_tag, delimiter, members, Vec::new())
            .map_err(|error| PyValueError::new_err(error.to_string()))
    }
}

#[pymethods]
impl Group {
    #[new]
    #[pyo3(signature = (tag, instance, /))]
    fn new(tag: u32, instance: Bound<'_, PyType>) -> PyResult<PyClassInitializer<Self>> {
        if !instance.is_subclass_of::<GroupInstance>()? {
            let name = instance.qualname()?;
            return Err(PyTypeError::new_err(format!(
                "'{name}' is not a subclass of GroupInstance"
            )));
        }
        let group = Self {
            instance: instance.unbind(),
        };
        Ok(PyClassInitializer::from(Field::new(tag)).add_subclass(group))
    }

    fn __get__<'py>(
        slf: &Bound<'py, Self>,
        obj: Option<&Bound<'py, PyAny>>,
        _owner: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Field::get_from(slf.as_super(), obj, |fields, _tag| {
            let py = fields.py();
            let table = Self::table(slf)?;
            let owner = DeclaredTags::of(&fields.get_type())?;
            let message = fields.get().message(py);
            let scope = fields.get().scope();
            let scopes = raw::group_scopes(py, &message, &scope, &table, &owner.known())?;

            let class = slf.get().instance.bind(py);
            let instances = scopes
                .unwrap_or_default()
                .into_iter()
                .map(|scope| class.call1((scope,)))
                .collect::<PyResult<Vec<_>>>()?;
            PyTuple::new(py, instances).map(Bound::into_any)
        })
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let name = slf.get_type().qualname()?;
        let tag = slf.as_super().get().tag();
        let instance = slf.get().instance.bind(slf.py()).qualname()?;
        Ok(format!("{name}({tag}, {instance})"))
    }
}
