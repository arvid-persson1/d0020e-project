//! A file containing all the structs and enums that build the GraphQL schema
use async_graphql::{Enum, InputObject, SimpleObject};
use std::str::FromStr;
use sqlx::{Type, FromRow};

// --- Needed for fetching ---
/// The representation of a book
// The book (isbn is used as identifier)
#[derive(SimpleObject, Clone, Debug, FromRow)]
pub(in crate) struct Book {
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
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub(in crate) enum BookFormatType {
    Pdf,
    Word,
    Epub,
    Hardcover,
    Paperback,
}

// A way to convert BookFormatType to strings in order to store in database
impl BookFormatType {
    pub fn as_string(&self) -> String {
        match self {
            BookFormatType::Pdf => "Pdf".to_string(),
            BookFormatType::Word => "Word".to_string(),
            BookFormatType::Epub => "Epub".to_string(),
            BookFormatType::Hardcover => "Hardcover".to_string(),
            BookFormatType::Paperback => "Paperback".to_string(),
        }
    }
}

// A way to convert string back to BookFormatType in order to fetch from database
impl FromStr for BookFormatType {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "Pdf" => Ok(BookFormatType::Pdf),
            "Word" => Ok(BookFormatType::Word),
            "Epub" => Ok(BookFormatType::Epub),
            "Hardcover" => Ok(BookFormatType::Hardcover),
            "Paperback" => Ok(BookFormatType::Paperback),
            // Error if nothing (shouldn't happen without typos, so I won't handle for now)
            _ => Err("It seems you typoed".to_string()),
        }
    }
}

// --- Needed for inserting ---
/// A representation of a book, but used specifically for inserts
#[derive(InputObject)]
pub(in crate) struct BookInput {
    pub isbn: String,
    pub title: String,
    pub author: String,
    pub format: BookFormatType,
}
