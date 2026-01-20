use async_graphql::{Enum, InputObject, SimpleObject};
use std::str::FromStr;

#[derive(SimpleObject, Clone, Debug)]
pub(crate) struct Book {
    pub isbn: String,
    pub title: String,
    pub author: String,
    pub format: BookFormatType,
}

// Unsure which of these are actually used/needed (LOOK UP LATER)
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum BookFormatType {
    Pdf,
    Word,
    Epub,
    Hardcover,
    Paperback,
}

// This exists to limit the different bookformats
impl BookFormatType {
    // This is my lazy way of making it easy to convert to
    fn as_string(&self) -> &str {
        match self {
            BookFormatType::Pdf => "Pdf",
            BookFormatType::Word => "Word",
            BookFormatType::Epub => "Epub",
            BookFormatType::Hardcover => "Hardcover",
            BookFormatType::Paperback => "Paperback",
        }
    }
}

// And this whole thing is just to make it simple to convert to BookFormatType
impl FromStr for BookFormatType {
    type Err = ();

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "Pdf" => Ok(BookFormatType::Pdf),
            "Word" => Ok(BookFormatType::Word),
            "Epub" => Ok(BookFormatType::Epub),
            "Hardcover" => Ok(BookFormatType::Hardcover),
            "Paperback" => Ok(BookFormatType::Paperback),
            // Error if nothing (shouldn't happen without typos, so I won't handle for now)
            _ => Err(()),
        }
    }
}

// Quite unsure if this is actually needed
// You'll also need to remove the EmptyMutation when creating an instance of the schema.
#[derive(InputObject)]
struct BookInput {
    isbn: String,
    title: String,
    author: String,
    format: BookFormatType,
}
