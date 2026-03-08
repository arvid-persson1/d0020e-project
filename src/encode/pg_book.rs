//! Decoder/Encoder for the postgres connector that uses diesel orm
use crate::postgres::models::Book;
use crate::postgres::schema::books; // Required for the encoder to find books::table!
use crate::postgres::{PgDecode, PgEncode}; // Adjust this path if your traits live elsewhere
use diesel::prelude::*;

/// The Postgres decoder/encoder for the Book model.
#[derive(Debug, Clone, Default)]
pub struct BookMapper;

impl PgDecode<Book> for BookMapper {
    /// Takes a DB connection and the SQL and decodes it into a list.
    #[inline]
    fn decode_all(&self, conn: &mut PgConnection, sql_text: &str) -> QueryResult<Vec<Book>> {
        let pg_safe_condition = sql_text.replace('\"', "'");

        let final_query = if pg_safe_condition.trim().is_empty() {
            "SELECT * FROM books".to_owned()
        } else {
            format!("SELECT * FROM books WHERE {pg_safe_condition}")
        };

        diesel::sql_query(final_query).load::<Book>(conn)
    }

    /// Fetches a single book, safely returning None if it doesn't exist.
    #[inline]
    fn decode_optional(
        &self,
        conn: &mut PgConnection,
        sql_text: &str,
    ) -> QueryResult<Option<Book>> {
        let pg_safe_condition = sql_text.replace('\"', "'");

        let final_query = if pg_safe_condition.trim().is_empty() {
            "SELECT * FROM books LIMIT 1".to_owned()
        } else {
            format!("SELECT * FROM books WHERE {pg_safe_condition} LIMIT 1")
        };

        diesel::sql_query(final_query)
            .get_result::<Book>(conn)
            .optional()
    }
}

impl PgEncode<Book> for BookMapper {
    /// Inserts a slice of multiple Books into the database.
    #[inline]
    fn encode_all(&self, conn: &mut PgConnection, entries: &[Book]) -> QueryResult<usize> {
        diesel::insert_into(books::table)
            .values(entries)
            .execute(conn)
    }

    /// Inserts a single Book into the database
    #[inline]
    fn encode_one(&self, conn: &mut PgConnection, entry: &Book) -> QueryResult<usize> {
        diesel::insert_into(books::table)
            .values(entry)
            .execute(conn)
    }

    /// Handles an optional insert, returning 0 rows affected if None
    #[inline]
    fn encode_optional(&self, conn: &mut PgConnection, entry: Option<&Book>) -> QueryResult<usize> {
        entry.map_or_else(|| Ok(0), |book| self.encode_one(conn, book))
    }
}
