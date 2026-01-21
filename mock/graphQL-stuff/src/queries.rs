//! This might be a missleading name, but all it's just a file that contains the different ways you interact via GraphQL e.g mutations or queries.
use crate::book_schema::{Book, BookFormatType};
use crate::db::DB;
use async_graphql::{Object, Result};

// Queries are for recieving data via GraphQl
pub(crate) struct Query {
    pub db: DB,
}

#[Object]
impl Query {
    // GraphQL query that returns book with corresponding isbn number
    async fn get_book(&self, isbn: String) -> Option<Book> {
        self.db.get_book(isbn).await
    }

    // GraphQL query that returns all books within the existing database
    async fn get_all_books(&self) -> Vec<Book> {
        self.db.get_all_books().await
    }
}

// While mutations are the for making writing or changing data via GraphQL
pub struct Mutation {
    pub db: DB,
}

#[Object]
impl Mutation {
    // GraphQL mutation that adds a book to the database (I chose to use BookFormatType here to avoid errors)
    async fn insert_book(
        &self,
        isbn: String,
        title: String,
        author: String,
        format: BookFormatType,
    ) -> Result<Book> {
        // Convert to string, then just call the database function that inserts
        let format_str = format.as_string();
        let book = self.db.insert_book(isbn, title, author, format_str).await?;
        // Return value
        Ok(book)
    }
}
