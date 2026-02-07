use crate::schema::books;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};

#[derive(DbEnum, Debug, PartialEq, Serialize, Deserialize, Clone)]
#[ExistingTypePath = "crate::schema::sql_types::BookFormatType"]
/// Type for the supported book formats
pub(crate) enum BookFormatType {
    /// Format for PDF
    #[db_rename = "Pdf"]
    Pdf,
    /// Format for docx (Word)
    #[db_rename = "Docx"]
    Docx,
    /// Format for Epub
    #[db_rename = "Epub"]
    Epub,
    /// Format for Hardcover
    #[db_rename = "Hardcover"]
    Hardcover,
    /// Format for Paperback
    #[db_rename = "Paperback"]
    Paperback,
}

#[derive(Queryable, Selectable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = books)]
#[diesel(check_for_backend(Pg))]
#[diesel(primary_key(isbn))]
/// The Book type
pub(crate) struct Book {
    /// The book title
    title: String,
    /// The book author
    author: String,
    /// The book format
    format: BookFormatType,
    /// The book isbn
    isbn: String,
}

impl Book {
    // This function only compiles when running 'cargo test'
    #[cfg(test)]
    pub(crate) fn get_title(&self) -> &str {
        &self.title
    }

    #[cfg(test)]
    pub(crate) fn get_author(&self) -> &str {
        &self.author
    }

    #[cfg(test)]
    pub(crate) fn get_format(&self) -> BookFormatType {
        self.format.clone()
    }

    #[cfg(test)]
    pub(crate) fn get_isbn(&self) -> &str {
        &self.isbn
    }
}
