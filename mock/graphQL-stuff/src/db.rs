//! A file containing all functions that are needed for the database
use crate::book_schema::{Book, BookFormatType, BookInput};
use sqlx::{Row, sqlite::SqlitePool};

#[derive(Clone)]
pub(in crate) struct DB {
    pub pool: SqlitePool,
}

impl DB {
    // Sets up a database on the provided db_path containing a table for books
    pub async fn new(db_path: &str) -> Self {
        let url = format!("sqlite:{}?mode=rwc", db_path);
        // This line makes me want to move to the top of a mountain and live in seclusion for five years.
        let pool = SqlitePool::connect(&url)
            .await
            .expect("Failed to connect to database");

        // This is to make sure the table actually exists, I hate that it gives me a warning
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS book (
                isbn TEXT PRIMARY KEY NOT NULL,
                title TEXT NOT NULL,
                author TEXT NOT NULL,
                format TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("Schema creation done diddly broke");

        // Return value (rust moment)
        Self { pool }
    }

    // Returns an array of all Books within the database
    pub async fn get_all_books(&self) -> Vec<Book> {
        sqlx::query_as::<_, Book>("SELECT isbn, title, author, format FROM book")
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
    }

    // Returns the first book with a matching isbn number within the database (the isbn is sensitive in that it has to be typed the exact way it's intended to)
    pub async fn get_book(&self, isbn: String) -> Option<Book> {
        sqlx::query_as::<_, Book>("SELECT isbn, title, author, format FROM book WHERE isbn = $1")
            .bind(isbn)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten()
    }

    /// Adds a book to the database, also returns the resulting book if it worked out
    pub(in crate) async fn insert_book(&self, book: BookInput) -> Result<Book, String> {
        sqlx::query("INSERT INTO book (isbn, title, author, format) VALUES (?, ?, ?, ?)")
            .bind(&book.isbn)
            .bind(&book.title)
            .bind(&book.author)
            .bind(book.format)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        // Return value (to know if it worked or not)
        Ok(Book {
            isbn: book.isbn,
            title: book.title,
            author: book.author,
            format: book.format,
        })
    }
}
