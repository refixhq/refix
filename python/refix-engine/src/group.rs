use pyo3::exceptions::PyValueError;
use pyo3::{Bound, PyResult, pyclass, pymethods};
use refix_message::{OwnedGroupTable, Tag};

#[pyclass(frozen, module = "refix._core")]
pub(crate) struct GroupTable(OwnedGroupTable);

#[pymethods]
impl GroupTable {
    #[new]
    fn new<'py>(
        count_tag: u32,
        delimiter: u32,
        members: Vec<u32>,
        nested: Vec<Bound<'py, GroupTable>>,
    ) -> PyResult<Self> {
        let members = members.into_iter().map(Tag).collect();
        let nested = nested.iter().map(|table| table.get().0.clone()).collect();
        OwnedGroupTable::new(Tag(count_tag), Tag(delimiter), members, nested)
            .map(Self)
            .map_err(|error| PyValueError::new_err(error.to_string()))
    }
}

#[pyclass(frozen, module = "refix._core")]
pub(crate) struct KnownTags(Vec<Tag>);

#[pymethods]
impl KnownTags {
    #[new]
    fn new(tags: Vec<u32>) -> PyResult<Self> {
        if !tags.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(PyValueError::new_err(
                "known tags must be sorted and unique",
            ));
        }
        Ok(Self(tags.into_iter().map(Tag).collect()))
    }
}
