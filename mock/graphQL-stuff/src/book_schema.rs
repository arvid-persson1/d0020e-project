//! A file containing all the structs and enums that build the GraphQL schema
use async_graphql::{Enum, SimpleObject};
use std::str::FromStr;

// --- Needed for fetching ---
// The book (isbn is used as identifier)
// NOTE TO SELF: Double check which of theese derive things are needed in everything
#[derive(SimpleObject, Clone, Debug)]
pub(crate) struct Book {
    pub isbn: String,
    pub title: String,
    pub author: String,
    pub format: BookFormatType,
}

// The type that limits bookformats
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum BookFormatType {
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

// NOTE TO SELF: You can create an InputObject, that (like it sounds) creates an object that's used for mutations instead of adding the values directly, but that shoulnd't be needed here.
