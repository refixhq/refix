use refix_dictionary::{DataType, Dictionary, Field, dictionary};

use crate::groups::is_generated;

/// How generated code reads a field's value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Conversion {
    /// Text, with `get_str`.
    Str,
    /// An integer, with `get_int`.
    Int,
    /// `Y` or `N`, with `get_bool`.
    Bool,
    /// A decimal, with `get_decimal`.
    Decimal,
    /// A date, with `get_date`.
    Date,
    /// A month and year, with `get_month_year`.
    MonthYear,
    /// A time of day, with `get_time`.
    Time,
    /// An instant in UTC, with `get_utc_timestamp`.
    UtcTimestamp,
    /// An enum over text, with `get_str`.
    StrEnum,
    /// An enum over integers, with `get_int`.
    IntEnum,
    /// Space-separated enum values, with `get_multiple_values`.
    MultipleValues,
    /// The bytes, under a `_raw` name.
    Raw,
}

impl Conversion {
    /// The conversion for `field`. Listed values take precedence over the type,
    /// but a BOOLEAN reads as `bool` whatever it lists.
    pub(crate) fn of(field: &Field) -> Self {
        let data_type = &field.data_type;
        if !field.values.is_empty() && *data_type != DataType::Boolean {
            return if data_type.is_multiple_value() {
                Self::MultipleValues
            } else if data_type.is_int_based() {
                Self::IntEnum
            } else {
                Self::StrEnum
            };
        }
        match data_type {
            DataType::String
            | DataType::Char
            | DataType::Currency
            | DataType::Exchange
            | DataType::Country
            | DataType::Language
            | DataType::Xid
            | DataType::XidRef => Self::Str,
            _ if data_type.is_int_based() => Self::Int,
            DataType::Boolean => Self::Bool,
            _ if data_type.is_float_based() => Self::Decimal,
            DataType::UtcDateOnly | DataType::LocalMktDate => Self::Date,
            DataType::MonthYear => Self::MonthYear,
            DataType::UtcTimeOnly => Self::Time,
            DataType::UtcTimestamp => Self::UtcTimestamp,
            _ => Self::Raw,
        }
    }

    /// Whether the value reads through a generated enum.
    pub(crate) const fn is_enum(self) -> bool {
        matches!(self, Self::StrEnum | Self::IntEnum | Self::MultipleValues)
    }
}

/// Whether an accessor generated from `dictionary` reads a field whose
/// conversion is `wanted`.
pub(crate) fn uses(dictionary: &Dictionary, wanted: impl Fn(Conversion) -> bool + Copy) -> bool {
    uses_in(dictionary.header().chain(dictionary.trailer()), wanted)
        || dictionary
            .messages()
            .any(|message| uses_in(message.members(), wanted))
}

/// Whether an accessor generated for `members` reads a field whose
/// conversion is `wanted`, inside group instances too.
fn uses_in<'a>(
    mut members: impl Iterator<Item = dictionary::Member<'a>>,
    wanted: impl Fn(Conversion) -> bool + Copy,
) -> bool {
    members.any(|member| match member {
        dictionary::Member::Field { field, .. } => wanted(Conversion::of(field)),
        dictionary::Member::Group(group) => is_generated(group) && uses_in(group.members(), wanted),
    })
}

#[cfg(test)]
mod tests {
    use super::Conversion;
    use refix_dictionary::{DataType, EnumValue, Field, Tag};

    fn field(data_type: DataType) -> Field {
        Field {
            name: "Field".to_owned(),
            tag: Tag(1),
            data_type,
            values: vec![],
        }
    }

    fn with_values(data_type: DataType) -> Field {
        let mut field = field(data_type);
        field.values = vec![EnumValue {
            value: "1".to_owned(),
            description: "ONE".to_owned(),
        }];
        field
    }

    #[test]
    fn strings_chars_and_their_subtypes_read_as_text() {
        for data_type in [
            DataType::String,
            DataType::Char,
            DataType::Currency,
            DataType::Exchange,
            DataType::Country,
            DataType::Language,
            DataType::Xid,
            DataType::XidRef,
        ] {
            assert_eq!(
                Conversion::of(&field(data_type.clone())),
                Conversion::Str,
                "{data_type:?}"
            );
        }
    }

    #[test]
    fn int_based_types_read_as_integers() {
        assert_eq!(Conversion::of(&field(DataType::Int)), Conversion::Int);
        assert_eq!(Conversion::of(&field(DataType::SeqNum)), Conversion::Int);
    }

    #[test]
    fn booleans_read_as_bools() {
        assert_eq!(Conversion::of(&field(DataType::Boolean)), Conversion::Bool);
    }

    #[test]
    fn a_boolean_reads_as_a_bool_whatever_it_lists() {
        assert_eq!(
            Conversion::of(&with_values(DataType::Boolean)),
            Conversion::Bool
        );
    }

    #[test]
    fn the_float_family_reads_as_decimals() {
        for data_type in [
            DataType::Float,
            DataType::Qty,
            DataType::Price,
            DataType::PriceOffset,
            DataType::Amt,
            DataType::Percentage,
        ] {
            assert_eq!(
                Conversion::of(&field(data_type.clone())),
                Conversion::Decimal,
                "{data_type:?}"
            );
        }
    }

    #[test]
    fn the_date_types_read_as_dates() {
        assert_eq!(
            Conversion::of(&field(DataType::UtcDateOnly)),
            Conversion::Date
        );
        assert_eq!(
            Conversion::of(&field(DataType::LocalMktDate)),
            Conversion::Date
        );
    }

    #[test]
    fn month_year_reads_as_a_month_year() {
        assert_eq!(
            Conversion::of(&field(DataType::MonthYear)),
            Conversion::MonthYear
        );
    }

    #[test]
    fn a_utc_time_reads_as_a_time() {
        assert_eq!(
            Conversion::of(&field(DataType::UtcTimeOnly)),
            Conversion::Time
        );
    }

    #[test]
    fn a_utc_timestamp_reads_as_a_utc_timestamp() {
        assert_eq!(
            Conversion::of(&field(DataType::UtcTimestamp)),
            Conversion::UtcTimestamp
        );
    }

    #[test]
    fn listed_values_read_through_an_enum() {
        assert_eq!(
            Conversion::of(&with_values(DataType::Char)),
            Conversion::StrEnum
        );
        assert_eq!(
            Conversion::of(&with_values(DataType::Int)),
            Conversion::IntEnum
        );
        assert_eq!(
            Conversion::of(&with_values(DataType::MultipleStringValue)),
            Conversion::MultipleValues
        );
    }

    #[test]
    fn other_types_read_raw() {
        assert_eq!(
            Conversion::of(&field(DataType::MultipleStringValue)),
            Conversion::Raw
        );
        assert_eq!(Conversion::of(&field(DataType::Data)), Conversion::Raw);
        assert_eq!(
            Conversion::of(&field(DataType::Other("CUSTOM".to_owned()))),
            Conversion::Raw
        );
    }

    #[test]
    fn only_enum_conversions_are_enums() {
        assert!(Conversion::StrEnum.is_enum());
        assert!(Conversion::IntEnum.is_enum());
        assert!(Conversion::MultipleValues.is_enum());
        assert!(!Conversion::Str.is_enum());
        assert!(!Conversion::Int.is_enum());
        assert!(!Conversion::Bool.is_enum());
        assert!(!Conversion::Decimal.is_enum());
        assert!(!Conversion::Date.is_enum());
        assert!(!Conversion::MonthYear.is_enum());
        assert!(!Conversion::Time.is_enum());
        assert!(!Conversion::UtcTimestamp.is_enum());
        assert!(!Conversion::Raw.is_enum());
    }
}
