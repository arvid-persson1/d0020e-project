//! JSON encoding.

use crate::{
    encode::{Decode, Encode},
    errors::{DecodeError, EncodeError},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{from_slice, to_vec};

/// An encoder and decoder for JSON.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Json;

impl Json {
    /// Format a value as a JSON bytestring.
    ///
    /// # Errors
    ///
    /// See [`serde_json::to_vec`].
    fn format<T>(value: &T) -> Result<Box<[u8]>, EncodeError>
    where
        T: ?Sized + Serialize,
    {
        to_vec(value).map(Into::into).map_err(|_err| todo!())
    }
}

impl<T> Encode<T> for Json
where
    T: Serialize,
{
    // NOTE: This implementation produces "compact" JSON with no whitespace, unless added by
    // `encode_one`.
    // PERF: This implementation allocates more than theoretically necessary. This could be avoided
    // if there was support for encoding into an existing buffer.
    #[inline]
    fn encode<'a, I>(&self, entries: I) -> Result<Box<[u8]>, EncodeError>
    where
        T: 'a,
        I: IntoIterator<Item = &'a T>,
    {
        let values = entries
            .into_iter()
            .map(|entry| self.encode_one(entry))
            .collect::<Result<Box<_>, _>>()?;

        let required_cap =
            // The combined length of the elements.
            values.iter().map(|value| value.len()).sum::<usize>()
            // The separating commas.
            + values.len() - 1
            // Brackets (`[]`).
            + 2;
        let mut buf = Vec::with_capacity(required_cap);

        let mut it = values.into_iter();
        buf.push(b'[');
        if let Some(next) = it.next() {
            buf.extend(next);
        }
        for value in it {
            buf.push(b',');
            buf.extend(value);
        }
        buf.push(b']');

        Ok(buf.into())
    }

    #[inline]
    fn encode_all(&self, entries: &[T]) -> Result<Box<[u8]>, EncodeError> {
        Self::format(entries)
    }

    #[inline]
    fn encode_one(&self, entry: &T) -> Result<Box<[u8]>, EncodeError> {
        Self::format(entry)
    }
}

impl<T> Decode<T> for Json
where
    T: DeserializeOwned,
{
    // TODO: `decode` can be overridden with a more efficient implementation, but it would require
    // implementing some functionality beyond what is provided by `serde_json`, or possibly just
    // managing a custom `Deserializer`.

    #[inline]
    fn decode_all(&self, bytes: &[u8]) -> Result<Vec<T>, DecodeError> {
        from_slice(bytes).map_err(|_err| todo!())
    }

    /// Decode a single entry from a slice, if one exists.
    ///
    /// This method poses no restriction on *which* entry should be returned. The format may
    /// however define an ordering.
    ///
    /// One entry is assumed to be fairly small such that collection all bytes into a slice is
    /// acceptable, and as such no stream variant of this method exists.
    #[inline]
    fn decode_optional(&self, bytes: &[u8]) -> Result<Option<T>, DecodeError> {
        if bytes.is_empty() {
            Ok(None)
        } else {
            from_slice(bytes).map(Some).map_err(|_err| todo!())
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::missing_panics_doc,
    reason = "Panics simply indicate failed tests."
)]
mod tests {
    use super::*;
    use crate::errors::*;
    use bytes::Bytes;
    use futures::{TryStreamExt as _, stream::iter as from_iter};
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct TestData {
        id: u32,
        name: String,
        tags: Vec<String>,
    }

    impl TestData {
        fn new(id: u32, name: &str) -> Self {
            Self {
                id,
                name: name.to_owned(),
                tags: vec!["tag1".to_owned(), "tag2".to_owned()],
            }
        }
    }

    #[test]
    fn encode_one() {
        let encoder = Json;
        let data = TestData::new(1, "test");

        let encoded = encoder.encode_one(&data);
        let encoded_bytes = encoded.unwrap();
        let decoded: Result<TestData, _> = from_slice(&encoded_bytes);

        assert_eq!(decoded.unwrap(), data);
    }

    #[test]
    fn encode_all() {
        let encoder = Json;
        let data = vec![
            TestData::new(1, "first"),
            TestData::new(2, "second"),
            TestData::new(3, "third"),
        ];

        let encoded = encoder.encode_all(&data);
        let encoded_bytes = encoded.unwrap();
        let decoded: Result<Vec<TestData>, _> = from_slice(&encoded_bytes);

        assert_eq!(decoded.unwrap(), data);
    }

    #[test]
    fn encode_with_iterator() {
        let encoder = Json;
        let data = vec![TestData::new(1, "one"), TestData::new(2, "two")];

        let encoded = encoder.encode(data.iter());
        let encoded_iter = encoded.unwrap();
        let encoded_all = encoder.encode_all(&data).unwrap();

        assert_eq!(encoded_iter, encoded_all);
    }

    #[test]
    fn decode_one() {
        let decoder = Json;
        let data = TestData::new(1, "one");

        let encoded = Json::format(&data).unwrap();
        let decoded: TestData = decoder.decode_one(&encoded).unwrap();

        assert_eq!(decoded, data);
    }

    #[test]
    fn decode_one_empty() {
        let decoder = Json;

        let result: Result<TestData, _> = decoder.decode_one(&[]);

        assert!(matches!(result.unwrap_err(), DecodeOneError::Empty));
    }

    #[test]
    fn decode_optional() {
        let decoder = Json;
        let data = TestData::new(1, "one");

        let encoded = Json::format(&data).unwrap();
        let decoded = decoder.decode_optional(&encoded);
        let empty_result: Result<Option<TestData>, _> = decoder.decode_optional(&[]);

        assert_eq!(decoded.unwrap(), Some(data));
        assert_eq!(empty_result.unwrap(), None);
    }

    #[test]
    fn decode_all() {
        let decoder = Json;
        let data = vec![
            TestData::new(1, "one"),
            TestData::new(2, "two"),
            TestData::new(3, "three"),
        ];

        let encoded = Json::format(&data).unwrap();
        let decoded: Result<Vec<TestData>, _> = decoder.decode_all(&encoded);

        assert_eq!(decoded.unwrap(), data);
    }

    #[test]
    fn encode_one_compact_json() {
        let encoder = Json;
        let data = TestData::new(0, "compact");

        let encoded = encoder.encode_one(&data).unwrap();
        let encoded_str = String::from_utf8_lossy(&encoded);

        // Should be compact (no unnecessary whitespace)
        assert!(!encoded_str.contains('\n'));
        assert!(!encoded_str.contains(' '));
    }

    #[test]
    fn encode_produces_valid_json_array() {
        let encoder = Json;
        let data = vec![TestData::new(1, "one"), TestData::new(2, "two")];

        let encoded = encoder.encode_all(&data).unwrap();
        let encoded_str = String::from_utf8_lossy(&encoded);

        assert!(encoded_str.starts_with('['));
        assert!(encoded_str.ends_with(']'));

        let parsed: Result<Value, _> = from_slice(&encoded);

        assert!(parsed.unwrap().is_array());
    }

    #[test]
    fn encode_iterator_vs_slice() {
        let encoder = Json;
        let data = vec![TestData::new(1, "same"), TestData::new(2, "different")];

        let encoded_from_slice = encoder.encode_all(&data).unwrap();
        let encoded_from_iter = encoder.encode(data.iter()).unwrap();

        assert_eq!(encoded_from_slice, encoded_from_iter);
    }

    #[tokio::test]
    async fn decode_stream() {
        let decoder = Json;
        let data = vec![TestData::new(1, "stream1"), TestData::new(2, "stream2")];

        let encoded = Json::format(&data).unwrap();
        let chunks: Vec<Result<Bytes, ConnectionError>> = vec![Ok(Bytes::from(encoded))];

        let stream = from_iter(chunks);
        let result_stream = Decode::<TestData>::decode(&decoder, stream).await;
        let items: Result<Vec<_>, _> = result_stream.unwrap().try_collect().await;

        assert_eq!(items.unwrap(), data);
    }
}
