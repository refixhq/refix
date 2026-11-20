use super::GroupInstance;
use super::field::Field;
use pyo3::exceptions::PyTypeError;
use pyo3::types::{PyAnyMethods, PyType, PyTypeMethods};
use pyo3::{Bound, Py, PyClassInitializer, PyResult, pyclass, pymethods};

/// A repeating group read as a tuple of its instances.
#[pyclass(extends = Field, frozen, module = "refix.fields")]
pub(crate) struct Group {
    instance: Py<PyType>,
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

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let name = slf.get_type().qualname()?;
        let tag = slf.as_super().get().tag();
        let instance = slf.get().instance.bind(slf.py()).qualname()?;
        Ok(format!("{name}({tag}, {instance})"))
    }
}
