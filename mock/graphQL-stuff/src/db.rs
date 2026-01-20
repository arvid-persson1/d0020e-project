use crate::book_schema::{Book, BookFormatType};
use sqlx::{Row, sqlite::SqlitePool};

#[derive(Clone)]
pub(crate) struct DB {
    pub pool: SqlitePool,
}

impl DB {
    pub async fn new(db_path: &str) -> Self {
        let url = format!("sqlite:{}?mode=rwc", db_path);
        // This line makes me want to move to the top of a mountain and live in seclusion for five years.
        let pool = SqlitePool::connect(&url)
            .await
            .expect("Failed to connect to database");

        // println!("The pool set up correctly");

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

    pub async fn get_all_books(&self) -> Vec<Book> {
        let rows = sqlx::query("SELECT isbn, title, author, format FROM book")
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        rows.into_iter()
            .map(|row| {
                // Unholy solution to convert strings BookFormatType
                let format_string: String = row.get("format");
                let format: BookFormatType = format_string.parse().unwrap();

                Book {
                    isbn: row.get("isbn"),
                    title: row.get("title"),
                    author: row.get("author"),
                    format,
                }
            })
            .collect()
    }
}
