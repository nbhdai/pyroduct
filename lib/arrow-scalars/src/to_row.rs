use crate::{ArrowRow, ArrowValue};

pub trait ToRow {
    /// Convert a borrowed reference to an ArrowRow with borrowed data.
    /// Field names are static strings from compile time.
    fn to_row(&self) -> ArrowRow<'_>;
}

pub trait ToValue {
    /// Convert a borrowed reference to an ArrowRow with borrowed data.
    /// Field names are static strings from compile time.
    fn to_value(&self) -> ArrowValue<'_>;
}

impl<'a> ToValue for String {
    fn to_value(&self) -> crate::ArrowValue<'_> {
        self.into()
    }
}

impl<'a, T: ToRow> ToValue for T {
    fn to_value(&self) -> crate::ArrowValue<'_> {
        ArrowValue::Group(self.to_row())
    }
}

impl<'a, T: ToRow> ToValue for Vec<T> {
    fn to_value(&self) -> crate::ArrowValue<'_> {
        ArrowValue::List(self.iter().map(|v| v.to_value()).collect())
    }
}

impl<'a, T: ToRow> ToValue for &'a [T] {
    fn to_value(&self) -> crate::ArrowValue<'_> {
        ArrowValue::List(self.iter().map(|v| v.to_value()).collect())
    }
}

impl<'a, T: ToRow> ToValue for Option<T> {
    fn to_value(&self) -> crate::ArrowValue<'_> {
        match self {
            Some(v) => v.to_value(),
            None => ArrowValue::Null,
        }
    }
}

impl<'a, T: ToRow> ToValue for &'a Option<T> {
    fn to_value(&self) -> crate::ArrowValue<'_> {
        match self {
            Some(v) => v.to_value(),
            None => ArrowValue::Null,
        }
    }
}
