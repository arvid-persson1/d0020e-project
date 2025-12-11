use async_graphql::{Enum, ID, InputObject, SimpleObject};

#[derive(SimpleObject, Clone)]
pub(crate) struct Book {
    pub id: ID,
    pub title: String,
    pub author: String,
    pub format: BookFormatType,
    pub isbn: String,
}

// Unsure which of these are actually used/needed (LOOK UP LATER)
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub(crate) enum BookFormatType {
    Pdf,
    Word,
    Epub,
    Hardcover,
    Paperback,
}

// Quite unsure if this is actually needed
// You'll also need to remove the EmptyMutation when creating an instance of the schema.
#[derive(InputObject)]
struct BookInput {
    title: String,
    author: String,
    format: BookFormatType,
    isbn: String,
}
