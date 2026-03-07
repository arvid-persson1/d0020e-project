//! Models file for book related types and for the diesel orm.
use super::schema::books;
use diesel::prelude::*;

#[derive(Debug, Eq, PartialEq, diesel_derive_enum::DbEnum)]
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
    Paperback,
    #[db_rename = "Pocket"]
    /// Pocket edition (PE)
    Pocket,
}

#[derive(QueryableByName, Insertable, Debug)]
#[diesel(table_name = books)]
/// The book struct
pub struct Book {
    /// Title
    pub title: String,
    /// Author
    pub author: String,
    /// Book format
    pub format: BookFormatType,
    /// isbn
    pub isbn: String,
}

#[derive(Debug, Clone, Default)]
/// Book decoder that uses diesel
pub struct BookDecoder;

impl BookDecoder {
    /// Takes a DB connection and the SQL and decodes it.
    ///
    /// # Errors
    ///
    /// Returns a `diesel::result::Error` if mapping the database rows to the `Book` struct fails.
    #[inline]
    pub fn decode_all(&self, conn: &mut PgConnection, sql_text: &str) -> QueryResult<Vec<Book>> {
        use diesel::sql_query;

        // Diesel uses schema.rs to validate that the columns match
        sql_query(sql_text).load::<Book>(conn)
    }
}

#[derive(Debug, Clone, Default)]
/// Book encoder that uses diesel
pub struct BookEncoder;

impl BookEncoder {
    /// Takes a slice of Books and safely inserts them into the database
    ///
    /// # Errors
    ///
    /// Returns a `diesel::result::Error` if the database rejects the insertion.
    #[inline]
    pub fn encode_all(&self, conn: &mut PgConnection, entries: &[Book]) -> QueryResult<usize> {
        diesel::insert_into(books::table)
            .values(entries)
            .execute(conn)
    }
}
