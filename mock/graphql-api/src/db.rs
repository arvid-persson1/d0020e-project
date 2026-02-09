//! A file containing all functions that are needed for the database
use crate::book_schema::{Book, BookInput};
use sqlx::sqlite::SqlitePool;

/// A struct representing the database
#[derive(Clone)]
pub(crate) struct Db {
    /// The connection to the database
    pub pool: SqlitePool,
}

impl Db {
    /// Sets up a database on the provided `db_path` containing a table for books
    /// # Panics
    /// Panics if the pool couldn't be set up correctly.
    pub(crate) async fn new(db_path: &str) -> Self {
        let url = format!("sqlite:{db_path}?mode=rwc");
        // This line makes me want to move to the top of a mountain and live in seclusion for five years.
        let pool = SqlitePool::connect(&url)
            .await
            .expect("Failed to connect to database");

        // This is to make sure the table actually exists. Note that I don't actually want to use the query result.
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS book (
                isbn TEXT PRIMARY KEY NOT NULL,
                title TEXT NOT NULL,
                author TEXT NOT NULL,
                format TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("Schema creation broke");

        // Return value (rust moment)
        Self { pool }
    }

    /// Returns an array of all Books within the database
    pub(crate) async fn get_all_books(&self) -> Vec<Book> {
        sqlx::query_as::<_, Book>("SELECT isbn, title, author, format FROM book")
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
    }

    /// Returns the book with a matching isbn number within the database. Note that:
    /// - Since the isbn is the primary key there's a maximum of one matching row/book.
    /// - The isbn number is syntax-sensitive, meaning it needs to be spelled the EXACT same way it is in the database.
    pub(crate) async fn get_book(&self, isbn: String) -> Option<Book> {
        sqlx::query_as::<_, Book>("SELECT isbn, title, author, format FROM book WHERE isbn = $1")
            .bind(isbn)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten()
    }

    /// Adds a book to the database, also returns the resulting book if it worked out
    /// # Errors
    /// Returns an error if the query didn't execute properly.
    pub(crate) async fn insert_book(&self, book: BookInput) -> Result<Book, String> {
        // Note that we don't want the query result since all we're doing is inserting
        let _ = sqlx::query("INSERT INTO book (isbn, title, author, format) VALUES (?, ?, ?, ?)")
            .bind(&book.isbn)
            .bind(&book.title)
            .bind(&book.author)
            .bind(book.format)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        // Return the input as a `Book` if everything worked out.
        Ok(Book {
            isbn: book.isbn,
            title: book.title,
            author: book.author,
            format: book.format,
        })
    }
}
