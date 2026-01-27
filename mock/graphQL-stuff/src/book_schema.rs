//! A file containing all the structs and enums that build the GraphQL schema
use async_graphql::{Enum, InputObject, SimpleObject};
use sqlx::{FromRow, Type};

// --- Needed for fetching ---
/// The representation of a book
// The book (isbn is used as identifier)
#[derive(SimpleObject, Clone, Debug, FromRow)]
pub(crate) struct Book {
    /// The isbn number of the book.
    pub isbn: String,
    /// The title of the book.
    pub title: String,
    /// The people who authored the book.
    pub author: String,
    /// The format of the book.
    pub format: BookFormatType,
}

/// Representation of the format of the book
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Type)]
// I was confused at first, but this just makes sqlx handle the enum as a lowercase string
// NOTE TO SELF: Why in the actual does this ALWAYS PRINT a warning?
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum BookFormatType {
    /// The format PDF.
    Pdf,
    /// The format Word.
    Word,
    /// The format EPUB.
    Epub,
    /// The format Hardcover.
    Hardcover,
    /// The format Paperback.
    Paperback,
}

// --- Needed for inserting ---
/// A representation of a book, but used specifically for inserts
#[derive(InputObject)]
pub(crate) struct BookInput {
    /// The isbn number of the book.
    pub isbn: String,
    /// The title of the book.
    pub title: String,
    /// The people who authored the book.
    pub author: String,
    /// The format of the book.
    pub format: BookFormatType,
}
