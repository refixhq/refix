use super::declared::declared;
use super::field::Field;
use super::{Fields, GroupInstance};
use crate::raw;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyAnyMethods, PyTuple, PyType, PyTypeMethods};
use pyo3::{Bound, Py, PyAny, PyClassInitializer, PyResult, pyclass, pymethods};
use refix_message::{OwnedGroupTable, Tag};

/// A repeating group read as a tuple of its instances.
#[pyclass(extends = Field, frozen, module = "refix.fields")]
pub(crate) struct Group {
    instance: Py<PyType>,
    table: PyOnceLock<OwnedGroupTable>,
}

impl Group {
    pub(super) fn instance(&self) -> &Py<PyType> {
        &self.instance
    }

    // Returns the table of the group, built from the fields of the instance class on first use.
    fn table<'a>(slf: &'a Bound<'_, Self>) -> PyResult<&'a OwnedGroupTable> {
        let py = slf.py();
        slf.get().table.get_or_try_init(py, || {
            let class = slf.get().instance.bind(py);
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
            let mut nested = Vec::new();
            for field in &fields {
                if let Ok(group) = field.cast::<Self>() {
                    nested.push(Self::table(group)?.clone());
                }
            }

            OwnedGroupTable::new(count_tag, delimiter, members, nested)
                .map_err(|error| PyValueError::new_err(error.to_string()))
        })
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
            table: PyOnceLock::new(),
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
            let known = Fields::known_tags(fields)?;
            let message = fields.get().message(py);
            let scope = fields.get().scope();
            let scopes = raw::group_scopes(py, &message, &scope, table, &known.get().known())?;

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
