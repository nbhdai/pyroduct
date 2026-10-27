#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;

/// A Clone-on-Write smart pointer that is either a `Box<T::Owned>`, a `&T`, or a `&'static T::Static`.
///
/// This provides `Cow`-like semantics but guarantees that the owned variant
/// is a heap-allocated `Box<T>`.
///
/// This is for self referencing
pub enum FieldName<'a> {
    /// Owned data stored in a Box.
    Owned(String),
    /// Borrowed data.
    Borrowed(&'a str),
    /// Static reference (zero allocation, 'static lifetime).
    Static(&'static str),
}

impl<'a> FieldName<'a> {
    pub fn reborrow(&self) -> FieldName<'_> {
        match self {
            FieldName::Owned(s) => FieldName::Borrowed(s.as_str()),
            FieldName::Borrowed(s) => FieldName::Borrowed(*s),
            FieldName::Static(s) => FieldName::Static(*s),
        }
    }
}

impl<'a> From<&'a str> for FieldName<'a> {
    fn from(s: &'a str) -> Self {
        FieldName::Borrowed(s)
    }
}

impl<'a> From<String> for FieldName<'a> {
    fn from(s: String) -> Self {
        FieldName::Owned(s)
    }
}

impl<'a> AsRef<str> for FieldName<'a> {
    fn as_ref(&self) -> &str {
        match self {
            FieldName::Owned(s) => s.as_str(),
            FieldName::Borrowed(s) => s,
            FieldName::Static(s) => s,
        }
    }
}

impl<'a> Clone for FieldName<'a> {
    fn clone(&self) -> Self {
        match self {
            FieldName::Owned(s) => FieldName::Owned(s.clone()),
            FieldName::Borrowed(s) => FieldName::Borrowed(s),
            FieldName::Static(s) => FieldName::Static(s),
        }
    }
}

impl<'a> FieldName<'a> {
    pub fn from_static(s: &'static str) -> Self {
        FieldName::Static(s)
    }

    pub fn to_static(&self) -> FieldName<'static> {
        match self {
            FieldName::Owned(s) => FieldName::Owned(s.clone()),
            FieldName::Borrowed(s) => FieldName::Owned(s.to_string()),
            FieldName::Static(s) => FieldName::Static(s),
        }
    }
}

impl<'a> Deref for FieldName<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

#[cfg(feature = "serde")]
impl<'a> Serialize for FieldName<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize exactly like a standard string
        serializer.serialize_str(self.as_ref())
    }
}

#[cfg(feature = "serde")]
impl<'a, 'de: 'a> Deserialize<'de> for FieldName<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FieldNameVisitor;

        impl<'de> de::Visitor<'de> for FieldNameVisitor {
            type Value = FieldName<'de>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string")
            }

            // Zero-copy path: The deserializer can give us a borrowed string.
            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(FieldName::Borrowed(v))
            }

            // Allocation path: The deserializer gives us a temporary reference.
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(FieldName::Owned(v.to_owned()))
            }

            // Owned path: The deserializer already allocated a String.
            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(FieldName::Owned(v))
            }
        }

        deserializer.deserialize_string(FieldNameVisitor)
    }
}

use std::cmp::Ordering;

impl<'a> PartialEq for FieldName<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl<'a> Eq for FieldName<'a> {}

impl<'a> PartialOrd for FieldName<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for FieldName<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}

// Optional but highly recommended: Allow comparisons with standard strings
impl<'a> PartialEq<str> for FieldName<'a> {
    fn eq(&self, other: &str) -> bool {
        self.as_ref() == other
    }
}

impl<'a> PartialEq<&str> for FieldName<'a> {
    fn eq(&self, other: &&str) -> bool {
        self.as_ref() == *other
    }
}

impl<'a> PartialEq<String> for FieldName<'a> {
    fn eq(&self, other: &String) -> bool {
        self.as_ref() == other.as_str()
    }
}

impl<'a> fmt::Display for FieldName<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_ref(), f)
    }
}

impl<'a> fmt::Debug for FieldName<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Formats with quotes, just like standard String/&str
        fmt::Debug::fmt(self.as_ref(), f)
    }
}

use std::borrow::{Borrow, Cow};
use std::hash::{Hash, Hasher};
use std::ops::Deref;

impl<'a> Hash for FieldName<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

impl<'a> Borrow<str> for FieldName<'a> {
    fn borrow(&self) -> &str {
        self.as_ref()
    }
}

impl<'a> Default for FieldName<'a> {
    fn default() -> Self {
        FieldName::Static("")
    }
}

// =============================================================================
// FieldNames<'a>
// =============================================================================

/// A Clone-on-Write container for a contiguous sequence of field names.
#[derive(Debug)]
pub enum FieldNames<'a> {
    Owned(Vec<Cow<'a, str>>),
    Borrowed(&'a [Cow<'a, str>]),
    Static(&'static [&'static str]),
    Empty(usize),
}

impl<'a> FieldNames<'a> {
    pub const fn empty(n: usize) -> Self {
        FieldNames::Empty(n)
    }

    pub fn len(&self) -> usize {
        match self {
            FieldNames::Owned(v) => v.len(),
            FieldNames::Borrowed(s) => s.len(),
            FieldNames::Static(s) => s.len(),
            FieldNames::Empty(l) => *l,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the name at `index` as `&str`.
    pub fn get(&self, index: usize) -> &str {
        match self {
            FieldNames::Owned(v) => &v[index],
            FieldNames::Borrowed(s) => &s[index],
            FieldNames::Static(s) => s[index],
            FieldNames::Empty(_) => "",
        }
    }

    /// Iterate all names as `&str`.
    pub fn iter(&self) -> FieldNamesIter<'_, 'a> {
        FieldNamesIter {
            names: self,
            pos: 0,
        }
    }

    /// Borrow a sub-range. Returns `Borrowed` for Owned/Borrowed,
    /// `Static` for Static.
    pub fn slice(&'a self, range: std::ops::Range<usize>) -> FieldNames<'a> {
        match self {
            FieldNames::Owned(v) => FieldNames::Borrowed(&v[range]),
            FieldNames::Borrowed(s) => FieldNames::Borrowed(&s[range]),
            FieldNames::Static(s) => FieldNames::Static(&s[range]),
            FieldNames::Empty(_) => FieldNames::Empty(range.end - range.start),
        }
    }

    pub fn to_static(&self) -> FieldNames<'static> {
        match self {
            FieldNames::Owned(v) => FieldNames::Owned(
                v.iter()
                    .map(|c| Cow::Owned(c.as_ref().to_owned()))
                    .collect(),
            ),
            FieldNames::Borrowed(s) => FieldNames::Owned(
                s.iter()
                    .map(|c| Cow::Owned(c.as_ref().to_owned()))
                    .collect(),
            ),
            FieldNames::Static(s) => FieldNames::Static(s),
            FieldNames::Empty(l) => FieldNames::Empty(*l),
        }
    }

    /// Convert to a fully owned `Vec<Cow<'a, str>>`, consuming self.
    pub fn into_owned_vec(self) -> Vec<Cow<'a, str>> {
        match self {
            FieldNames::Owned(v) => v,
            FieldNames::Borrowed(s) => s.to_vec(),
            FieldNames::Static(s) => s.iter().map(|&r| Cow::Borrowed(r)).collect(),
            FieldNames::Empty(l) => vec!["".into(); l],
        }
    }
}

impl<'a> Clone for FieldNames<'a> {
    fn clone(&self) -> Self {
        match self {
            FieldNames::Owned(v) => FieldNames::Owned(v.clone()),
            FieldNames::Borrowed(s) => FieldNames::Borrowed(s),
            FieldNames::Static(s) => FieldNames::Static(s),
            FieldNames::Empty(l) => FieldNames::Empty(*l),
        }
    }
}

impl<'a> PartialEq for FieldNames<'a> {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl<'a> Eq for FieldNames<'a> {}

impl<'a> std::hash::Hash for FieldNames<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.len().hash(state);
        for name in self.iter() {
            name.hash(state);
        }
    }
}

impl<'a> From<Vec<Cow<'a, str>>> for FieldNames<'a> {
    fn from(v: Vec<Cow<'a, str>>) -> Self {
        FieldNames::Owned(v)
    }
}

impl From<Vec<String>> for FieldNames<'static> {
    fn from(v: Vec<String>) -> Self {
        FieldNames::Owned(v.into_iter().map(Cow::Owned).collect())
    }
}

impl<'a> From<&'a [Cow<'a, str>]> for FieldNames<'a> {
    fn from(s: &'a [Cow<'a, str>]) -> Self {
        FieldNames::Borrowed(s)
    }
}

impl From<&'static [&'static str]> for FieldNames<'static> {
    fn from(s: &'static [&'static str]) -> Self {
        FieldNames::Static(s)
    }
}

impl<'a> Default for FieldNames<'a> {
    fn default() -> Self {
        FieldNames::empty(0)
    }
}

// =============================================================================
// FieldNamesIter
// =============================================================================

pub struct FieldNamesIter<'f, 'a> {
    names: &'f FieldNames<'a>,
    pos: usize,
}

impl<'f, 'a> Iterator for FieldNamesIter<'f, 'a> {
    type Item = &'f str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.names.len() {
            return None;
        }
        let s = self.names.get(self.pos);
        self.pos += 1;
        Some(s)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let r = self.names.len() - self.pos;
        (r, Some(r))
    }
}

impl<'f, 'a> ExactSizeIterator for FieldNamesIter<'f, 'a> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    #[cfg(feature = "human")]
    fn test_serde_serialization() {
        let static_field = FieldName::from_static("test_static");
        let serialized = serde_json::to_string(&static_field).unwrap();
        assert_eq!(serialized, r#""test_static""#);
    }

    #[test]
    #[cfg(feature = "human")]
    fn test_serde_deserialization_borrowed() {
        // A raw JSON string without escape characters allows serde_json to yield a borrowed &str
        let json_data = r#""zero_copy_string""#;
        let deserialized: FieldName = serde_json::from_str(json_data).unwrap();

        // Verify that it successfully used the zero-copy borrowed variant
        assert!(
            matches!(deserialized, FieldName::Borrowed("zero_copy_string")),
            "Expected FieldName::Borrowed, but got {:?}",
            deserialized
        );
    }

    #[test]
    #[cfg(feature = "human")]
    fn test_serde_deserialization_owned() {
        // A JSON string with escape characters forces serde_json to allocate a new String
        let json_data = r#""allocated\nstring""#;
        let deserialized: FieldName = serde_json::from_str(json_data).unwrap();

        // Verify that it fell back to the owned variant due to the escape sequence
        match deserialized {
            FieldName::Owned(s) => assert_eq!(s, "allocated\nstring"),
            _ => panic!("Expected FieldName::Owned, but got {:?}", deserialized),
        }
    }

    #[test]
    fn test_comparisons() {
        let static_field = FieldName::from_static("hello");
        let owned_field = FieldName::from("hello".to_string());
        let borrowed_field = FieldName::from("hello");

        // They should all equal each other regardless of the internal variant
        assert_eq!(static_field, owned_field);
        assert_eq!(owned_field, borrowed_field);

        // They should easily compare to standard strings
        assert_eq!(static_field, "hello");
        assert_eq!(owned_field, "hello".to_string());
    }

    #[test]
    fn test_hashmap_compatibility() {
        let mut map = HashMap::new();

        // Insert using the cheap static variant
        map.insert(FieldName::from_static("id"), 42);
        map.insert(FieldName::from_static("name"), 99);

        // Because we implemented `Borrow<str>`, we can query the map directly with a standard &str
        assert_eq!(map.get("id"), Some(&42));
        assert_eq!(map.get("name"), Some(&99));
        assert_eq!(map.get("missing"), None);
    }

    #[test]
    fn test_default() {
        let default_field = FieldName::default();

        // Default should be a static empty string
        assert!(matches!(default_field, FieldName::Static("")));
        assert_eq!(default_field.as_ref(), "");
    }

    #[test]
    fn test_formatting() {
        let field = FieldName::from_static("format_me");

        // Test Display
        assert_eq!(format!("{}", field), "format_me");

        // Test Debug (should include quotes)
        assert_eq!(format!("{:?}", field), r#""format_me""#);
    }
}
