//! A file containing all the structs and enums that build the GraphQL schema
use async_graphql::{Enum, InputObject, SimpleObject};
use sqlx::{FromRow, Type};

// TODO: Maybe figure out how to auto generate this schema based on inital input.

// --- Needed for fetching ---
/// The representation of a book
// The book (isbn is used as identifier)
#[derive(SimpleObject, Clone, Debug, FromRow)]
pub(crate) struct Book {
    /// The isbn number of the book.
    pub(crate) isbn: String,
    /// The title of the book.
    pub(crate) title: String,
    /// The people who authored the book.
    pub(crate) author: String,
    /// The format of the book.
    pub(crate) format: BookFormatType,
}

/// Representation of the format of the book
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Type)]
// I was confused at first, but this just makes sqlx handle the enum as a lowercase string
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub(crate) enum BookFormatType {
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
/// A struct representing the possible values of a book.
/// Used for filtering queries and inserting books.
#[derive(InputObject, Clone, Debug, FromRow)]
pub(crate) struct FilteredBookInput {
    /// The existance (or not) of an isbn number of the book.
    pub(crate) isbn: Option<String>,
    /// The existance (or not) of a title of the book.
    pub(crate) title: Option<String>,
    /// The existance (or not) of an author of the book.
    pub(crate) author: Option<String>,
    /// The existance (or not) of a format of the book.
    pub(crate) format: Option<BookFormatType>,
}
