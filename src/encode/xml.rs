//! XML encoder/decoder.

use crate::{
    encode::{Decode, Encode},
    errors::{DecodeError, EncodeError},
};

use quick_xml::de::from_reader;
use quick_xml::se::to_string;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// The XML encoding/decoding
/// # Errors
/// Returns an error if value cannot be serialized into XML.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Xml;

impl Xml {
    /// Formats values as xml bytestrings
    /// # Errors
    /// Returns an error if XML serialization fails.
    fn format<T>(value: &T) -> Result<Box<[u8]>, EncodeError>
    where
        T: ?Sized + Serialize,
    {
        to_string(value)
            .map(|s| s.into_bytes().into_boxed_slice())
            .map_err(|err| EncodeError(Box::new(err)))
    }
}

impl<T> Encode<T> for Xml
where
    T: Serialize,
{
    #[inline]
    fn encode<'a, I>(&self, entries: I) -> Result<Box<[u8]>, EncodeError>
    where
        T: 'a,
        I: IntoIterator<Item = &'a T>,
    {
        let vals: Result<Vec<String>, _> =
            entries.into_iter().map(|entry| to_string(entry)).collect();

        let vals = vals.map_err(|err| EncodeError(Box::new(err)))?;

        let start_tag = b"<List>";
        let end_tag = b"</List>";

        let content_len: usize = vals.iter().map(String::len).sum();
        let required_cap = start_tag.len() + content_len + end_tag.len();

        let mut buf = Vec::with_capacity(required_cap);

        // Constructs the XML document
        buf.extend_from_slice(start_tag);
        for val in vals {
            buf.extend_from_slice(val.as_bytes());
        }
        buf.extend_from_slice(end_tag);

        Ok(buf.into_boxed_slice())
    }
    #[inline]
    fn encode_all(&self, entries: &[T]) -> Result<Box<[u8]>, EncodeError> {
        self.encode(entries)
    }

    #[inline]
    fn encode_one(&self, entry: &T) -> Result<Box<[u8]>, EncodeError> {
        Self::format(entry)
    }
}

impl<T> Decode<T> for Xml
where
    T: DeserializeOwned,
{
    #[inline]
    fn decode_all(&self, bytes: &[u8]) -> Result<Vec<T>, DecodeError> {
        #[derive(Deserialize)]
        struct ListWrapper<T> {
            #[serde(rename = "$value")]
            items: Vec<T>,
        }

        let wrapper: ListWrapper<T> =
            from_reader(bytes).map_err(|err| DecodeError(Box::new(err)))?;

        Ok(wrapper.items)
    }

    #[inline]
    fn decode_optional(&self, bytes: &[u8]) -> Result<Option<T>, DecodeError> {
        if bytes.is_empty() {
            Ok(None)
        } else {
            from_reader(bytes)
                .map(Some)
                .map_err(|err| DecodeError(Box::new(err)))
        }
    }
}

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "Panics when tests fail")]
#[allow(clippy::unwrap_used, reason = "Panics when tests fail")]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::vec;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename = "TestData")]
    struct TestData {
        id: u64,
        name: String,
        tags: Vec<String>,
    }

    impl TestData {
        fn new(id: u64, name: &str) -> Self {
            Self {
                id,
                name: name.to_owned(),
                tags: vec![format!("{name}-tag1"), format!("{name}-tag2")],
            }
        }
    }

    #[test]
    fn encode_one() {
        let encoder = Xml;
        let data = TestData::new(1, "test");

        let encoded = encoder.encode_one(&data);
        let encoded_bytes = encoded.unwrap();

        let encoded_str = String::from_utf8(encoded_bytes.to_vec()).unwrap();
        assert!(encoded_str.contains("<TestData>"));
        assert!(encoded_str.contains("<id>1</id>"));

        let decoder = Xml;
        let decoded: Result<TestData, _> = decoder.decode_one(&encoded_bytes);
        assert_eq!(decoded.unwrap(), data);
    }

    #[test]
    fn encode_all() {
        let encoder = Xml;
        let data = vec![
            TestData::new(1, "first"),
            TestData::new(2, "second"),
            TestData::new(3, "third"),
        ];

        let encoded = encoder.encode_all(&data);
        let encoded_bytes = encoded.unwrap();

        let decoder = Xml;
        let decoded: Result<Vec<TestData>, _> = decoder.decode_all(&encoded_bytes);
        assert_eq!(decoded.unwrap(), data);
    }

    #[test]
    fn encode_all_iterators() {
        let encoder = Xml;
        let data = vec![TestData::new(1, "one"), TestData::new(2, "two")];

        let encoded_vec = encoder.encode_all(&data).unwrap();
        let encoded_iter = encoder.encode(data.iter()).unwrap();
        let encoded_all = encoder.encode(data.iter()).unwrap();

        assert_eq!(encoded_vec, encoded_iter);
        assert_eq!(encoded_vec, encoded_all);
    }

    #[test]
    fn decode_one() {
        let encoder = Xml;
        let data = TestData::new(1, "test");
        let encoded = encoder.encode_one(&data).unwrap();

        let decoder = Xml;
        let decoded: TestData = decoder.decode_one(&encoded).unwrap();

        assert_eq!(decoded, data);
    }

    #[test]
    fn decode_one_empty() {
        use crate::errors::DecodeOneError;
        let decoder = Xml;
        let res: Result<TestData, _> = decoder.decode_one(&[]);
        assert!(matches!(res.unwrap_err(), DecodeOneError::Empty));
    }

    #[test]
    fn decode_optional() {
        let encoder = Xml;
        let data = TestData::new(1, "test");

        let encoded = encoder.encode_one(&data).unwrap();

        let decoder = Xml;
        let decoded: Option<TestData> = decoder.decode_optional(&encoded).unwrap();
        let empty: Option<TestData> = decoder.decode_optional(&[]).unwrap();

        assert_eq!(decoded.unwrap(), data);
        assert!(empty.is_none());
    }

    #[test]
    fn decode_all() {
        let encoder = Xml;
        let data = vec![
            TestData::new(1, "one"),
            TestData::new(2, "two"),
            TestData::new(3, "three"),
        ];

        let encoded = encoder.encode_all(&data).unwrap();
        let decoder = Xml;
        let decoded: Vec<TestData> = decoder.decode_all(&encoded).unwrap();

        assert_eq!(decoded, data);
    }

    #[test]
    fn encoded_one_compact_xml() {
        let encoder = Xml;
        let data = TestData::new(1, "compact");

        let encoded = encoder.encode_one(&data).unwrap();
        let encoded_str = String::from_utf8(encoded.to_vec()).unwrap();

        println!("Output XML: {encoded_str}");

        assert!(!encoded_str.contains('\n'));
        assert!(encoded_str.starts_with("<TestData>"));
    }

    #[test]
    fn encode_produces_valid_xml_list() {
        let encoder = Xml;
        let data = vec![TestData::new(1, "one"), TestData::new(2, "two")];

        let encoded = encoder.encode_all(&data).unwrap();
        let encoded_str = String::from_utf8(encoded.to_vec()).unwrap();

        assert!(encoded_str.starts_with("<List>"));
        assert!(encoded_str.ends_with("</List>"));

        assert!(encoded_str.contains("<TestData>"));
    }
}
