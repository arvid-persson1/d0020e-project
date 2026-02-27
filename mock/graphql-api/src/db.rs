//! A file containing all functions that are needed for the database
use crate::book_schema::{Book, FilteredBookInput};
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
                isbn TEXT NOT NULL,
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

    /// A query that returns an array of books matching the input.
    // NOTE: sqlx just removes the Some and looks for the values instead.
    pub(crate) async fn get_books(&self, filter: FilteredBookInput) -> Vec<Book> {
        // This is the fastest way I came up with in a short time.
        sqlx::query_as::<_, Book>(
            "SELECT isbn, title, author, format FROM book 
            WHERE ($1 IS NULL OR isbn = $1)
            AND ($2 IS NULL OR title = $2)
            AND ($3 IS NULL OR author = $3)
            AND ($4 IS NULL OR format = $4)",
        )
        .bind(filter.isbn)
        .bind(filter.title)
        .bind(filter.author)
        .bind(filter.format)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
    }

    /// Adds a book to the database, also returns the resulting book if it worked out
    /// # Errors
    /// Returns an error if the query didn't execute properly.
    pub(crate) async fn insert_book(&self, book: FilteredBookInput) -> Result<Book, String> {
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
            isbn: book.isbn.expect("Somehow there was no value"),
            title: book.title.expect("Somehow there was no value"),
            author: book.author.expect("Somehow there was no value"),
            format: book.format.expect("Somehow there was no value"),
        })
    }
}
