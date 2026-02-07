//! Placeholder module.

/// Placeholder trait.
// TODO: Replace with actual query type.
pub trait Query<T> {}

/// Placeholder for where a translator is to be inserted.
#[inline]
#[must_use]
#[expect(clippy::unimplemented, reason = "TODO")]
pub fn translate<T>(_: &dyn Query<T>) -> Never {
    unimplemented!("Call proper translator implementation.")
}

#[expect(missing_debug_implementations, reason = "TODO")]
#[expect(missing_docs, reason = "TODO")]
pub struct Never(!);

impl serde::Serialize for Never {
    #[inline]
    #[expect(clippy::unimplemented, reason = "TODO")]
    fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        unimplemented!()
    }
}
