//! Models file for book related types and for the diesel orm.
use super::schema::books;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Eq, PartialEq, Hash, diesel_derive_enum::DbEnum, Serialize, Deserialize, Default,
)]
#[ExistingTypePath = "crate::postgres::schema::sql_types::BookFormatType"]
/// The type for format
pub enum BookFormatType {
    #[db_rename = "Pdf"]
    /// Pdf
    Pdf,
    #[db_rename = "Docx"]
    /// Docx (Word)
    Docx,
    #[db_rename = "Epub"]
    /// Epub
    Epub,
    #[db_rename = "Hardcover"]
    /// Hardcover
    Hardcover,
    #[db_rename = "Paperback"]
    /// Paperback
    #[default]
    Paperback,
    #[db_rename = "Pocket"]
    /// Pocket edition (PE)
    Pocket,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    diesel::Queryable,
    diesel::QueryableByName,
    diesel::Selectable,
    diesel::Insertable,
    crate::query::Queryable,
)]
#[diesel(table_name = books)]
/// The book struct
pub struct Book {
    /// Title
    pub title: String,
    /// Author
    pub author: String,
    /// Book format
    #[serde(default)]
    pub format: BookFormatType,
    /// isbn
    pub isbn: String,
}
