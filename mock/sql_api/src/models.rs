use crate::schema::books;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel_derive_enum::DbEnum;
use serde::Serialize;

#[derive(DbEnum, Debug, PartialEq, Serialize)]
#[ExistingTypePath = "crate::schema::sql_types::BookFormatType"]
/// Type for the supported book formats
pub(crate) enum BookFormatType {
    /// Format for PDF
    Pdf,
    /// Format for docx (Word)
    Docx,
    /// Format for Epub
    Epub,
    /// Format for Hardcover
    Hardcover,
    /// Format for Paperback
    Paperback,
}

#[derive(Queryable, Selectable, Serialize, Debug)]
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
