//! XML encoder/decoder.

use crate::{
  encode::{Encode, Decode},
  errors::{EncodeError, DecodeError},
};

use serde::{Serialize, de::DeserializeOwned};
use quick_xml::se::to_string;
use serde_json::from_slice;

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
    let vals: Result<Vec<String>, _> = entries
      .into_iter()
      .map(|entry| to_string(entry))
      .collect();

    let vals = vals.map_err(|err| EncodeError(Box::new(err)))?;

    let start_tag=b"<List>";
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
    Self::format(entries)
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
    from_slice(bytes).map_err(|err| DecodeError(Box::new(err)))
  }

  #[inline]
  fn decode_optional(&self, bytes: &[u8]) -> Result<Option<T>, DecodeError> {
      if bytes.is_empty() {
        Ok(None)
      } else {
        from_slice(bytes)
          .map(Some)
        .map_err(|err| DecodeError(Box::new(err)))
      }
  }
}
