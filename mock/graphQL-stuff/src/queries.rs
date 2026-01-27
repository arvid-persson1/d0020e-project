//! This might be a missleading name, but all it's just a file that contains the different ways you interact via GraphQL e.g mutations or queries.
use crate::book_schema::{Book, BookInput};
use crate::db::Db;
use async_graphql::{Object, Result};

/// Struct used for GraphQL queries.
pub struct Query {
    /// The database that's used in the queries.
    pub db: Db,
}

#[Object]
impl Query {
    /// GraphQL query that returns book with corresponding isbn number.
    ///
    /// Queries look might look like this:
    ///     query {
    ///     getBook(isbn: "1") {
    /// 	    title
    /// 	    author
    /// 	    format
    ///     }
    /// }
    async fn get_book(&self, isbn: String) -> Option<Book> {
        self.db.get_book(isbn).await
    }

    /// GraphQL query that returns all books within the existing database
    ///
    ///     query {
    ///     getAllBooks {
    /// 	    title
    /// 	    author
    /// 	    format
    ///     }
    /// }
    async fn get_all_books(&self) -> Vec<Book> {
        self.db.get_all_books().await
    }
}

/// Struct used for GraphQL mutations
pub(crate) struct Mutation {
    /// The database that's used in the mutations.
    pub db: Db,
}

#[Object]
impl Mutation {
    /// GraphQL mutation that adds a book to the database. `BookFormatType` was chosen here to avoid errors).
    ///
    /// When sending making the request you can write something like:
    /// mutation {
    ///     insertBook(book: [
    /// 	{ isbn: "1", title: "Book 1", author: "Author A", format: HARDCOVER },
    ///     { isbn: "2", title: "Book 2", author: "Author B", format: PAPERBACK },
    ///     { isbn: "3", title: "Book 3", author: "Author C", format: EPUB },
    ///     ]) {
    ///         title
    ///         isbn
    ///     }
    /// }
    async fn insert_book(&self, book: BookInput) -> Result<Book> {
        // Book is shadowed here since we want values to return aswell.
        let book = self.db.insert_book(book).await?;
        // Convert to book since that's the actual object
        Ok(book)
    }

    /// GraphQL mutation that adds multiple books to the database
    ///
    /// When sending making the request you can write something like:
    /// mutation {
    ///     insertBook(book: {isbn: "2", title: "thatBook", author: "Shakespeare", format: HARDCOVER}) {
    ///	     format
    ///     }
    /// }
    async fn insert_books(&self, books: Vec<BookInput>) -> Result<Vec<Book>> {
        let mut inserted = vec![];

        for book in books {
            let current_book = self.db.insert_book(book).await?;
            // Add the inserted book to array of inserted books
            inserted.push(current_book);
        }

        // Return array of all inserted books
        Ok(inserted)
    }
}
