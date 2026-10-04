use crate::{InvalidValue, RawMessage, Scope, Tag};
use std::fmt;
use std::iter::FusedIterator;
use std::marker::PhantomData;

/// The values of a multiple-value field, such as `ExecInst(18)`.
pub struct MultipleValues<'a, T> {
    raw: &'a str,
    value: PhantomData<fn() -> T>,
}

impl<'a, T: From<&'a str>> MultipleValues<'a, T> {
    /// The values, in wire order, duplicates included.
    pub fn iter(&self) -> Iter<'a, T> {
        Iter {
            values: self.raw.split(' '),
            value: PhantomData,
        }
    }

    /// Whether any of the values is `value`.
    pub fn contains(&self, value: T) -> bool
    where
        T: PartialEq,
    {
        self.iter().any(|item| item == value)
    }
}

impl<T> Clone for MultipleValues<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for MultipleValues<'_, T> {}

impl<T> fmt::Debug for MultipleValues<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.raw.split(' ')).finish()
    }
}

impl<'a, T: From<&'a str>> IntoIterator for &MultipleValues<'a, T> {
    type Item = T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over a field's values, each read through `T`.
pub struct Iter<'a, T> {
    values: std::str::Split<'a, char>,
    value: PhantomData<fn() -> T>,
}

impl<'a, T: From<&'a str>> Iterator for Iter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.values.next().map(T::from)
    }
}

impl<'a, T: From<&'a str>> FusedIterator for Iter<'a, T> {}

impl RawMessage {
    /// The values of a multiple-value field, read from the whole frame.
    pub fn get_multiple_values<T>(
        &self,
        tag: Tag,
    ) -> Result<Option<MultipleValues<'_, T>>, InvalidValue> {
        self.scope().get_multiple_values(tag)
    }
}

impl<'a> Scope<'a> {
    /// The values of a multiple-value field, or `Ok(None)` when absent.
    ///
    /// The field must hold one or more values separated by single spaces.
    pub fn get_multiple_values<T>(
        &self,
        tag: Tag,
    ) -> Result<Option<MultipleValues<'a, T>>, InvalidValue> {
        let Some(raw) = self.get_str(tag)? else {
            return Ok(None);
        };
        if raw.split(' ').any(str::is_empty) {
            return Err(InvalidValue { tag });
        }
        Ok(Some(MultipleValues {
            raw,
            value: PhantomData,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::construct_valid_frame;
    use crate::{GroupTable, KnownTags, Tokenizer};
    use bytes::Bytes;

    /// A FIX.4.4 frame around the `|`-delimited `body`.
    fn message(body: &str) -> RawMessage {
        let frame = construct_valid_frame("FIX.4.4", body);
        Tokenizer::default().tokenize(Bytes::from(frame)).unwrap()
    }

    fn exec_inst(message: &RawMessage) -> MultipleValues<'_, &str> {
        message.get_multiple_values(Tag(18)).unwrap().unwrap()
    }

    #[test]
    fn reads_values_in_wire_order_with_duplicates() {
        let message = message("35=D|18=1 6 1|");
        let values: Vec<&str> = exec_inst(&message).iter().collect();
        assert_eq!(values, ["1", "6", "1"]);
    }

    #[test]
    fn reads_a_single_value() {
        let message = message("35=D|18=G|");
        let values: Vec<&str> = exec_inst(&message).iter().collect();
        assert_eq!(values, ["G"]);
    }

    #[test]
    fn an_absent_field_has_no_values() {
        let message = message("35=D|");
        assert!(matches!(
            message.get_multiple_values::<&str>(Tag(18)),
            Ok(None)
        ));
    }

    #[test]
    fn anything_but_single_spaces_between_values_is_an_error() {
        for body in ["35=D|18=|", "35=D|18= 1|", "35=D|18=1 |", "35=D|18=1  6|"] {
            let message = message(body);
            assert_eq!(
                message.get_multiple_values::<&str>(Tag(18)).unwrap_err(),
                InvalidValue { tag: Tag(18) },
                "{body}"
            );
        }
    }

    #[test]
    fn contains_looks_for_a_value() {
        let message = message("35=D|18=1 6|");
        let values = exec_inst(&message);
        assert!(values.contains("6"));
        assert!(!values.contains("2"));
    }

    #[test]
    fn values_are_read_through_their_type() {
        #[derive(Debug, PartialEq)]
        enum ExecInst<'a> {
            NotHeld,
            Unrecognized(&'a str),
        }

        impl<'a> From<&'a str> for ExecInst<'a> {
            fn from(value: &'a str) -> Self {
                match value {
                    "1" => Self::NotHeld,
                    unrecognized => Self::Unrecognized(unrecognized),
                }
            }
        }

        let message = message("35=D|18=1 Z|");
        let values: MultipleValues<'_, ExecInst<'_>> =
            message.get_multiple_values(Tag(18)).unwrap().unwrap();

        let read: Vec<ExecInst<'_>> = (&values).into_iter().collect();
        assert_eq!(read, [ExecInst::NotHeld, ExecInst::Unrecognized("Z")]);
    }

    #[test]
    fn debug_lists_the_values() {
        let message = message("35=D|18=1 6|");
        assert_eq!(format!("{:?}", exec_inst(&message)), r#"["1", "6"]"#);
    }

    #[test]
    fn an_instance_reads_only_its_own_values() {
        const TABLE: GroupTable<'static> =
            GroupTable::new(Tag(453), Tag(448), &[Tag(18), Tag(448)], &[]);
        const KNOWN: KnownTags<'static> = KnownTags::new(&[Tag(18), Tag(35), Tag(448), Tag(453)]);

        let message = message("35=D|453=2|448=A|18=1 6|448=B|");
        let group = message.get_group(&TABLE, &KNOWN).unwrap().unwrap();
        let instances: Vec<Scope<'_>> = group.iter().collect();

        let first: MultipleValues<'_, &str> =
            instances[0].get_multiple_values(Tag(18)).unwrap().unwrap();
        assert_eq!(first.iter().collect::<Vec<_>>(), ["1", "6"]);
        assert!(matches!(
            instances[1].get_multiple_values::<&str>(Tag(18)),
            Ok(None)
        ));
    }
}
