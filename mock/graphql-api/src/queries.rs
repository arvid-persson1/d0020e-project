//! This might be a missleading name, but all it's just a file that contains the different ways you interact via GraphQL e.g mutations or queries.
use crate::{
    book_schema::{Book, FilteredBookInput},
    db::Db,
};
use async_graphql::{Object, Result};

/// Struct used for GraphQL queries.
pub(crate) struct Query {
    /// The database that's used in the queries.
    pub db: Db,
}

#[Object]
impl Query {
    /// GraphQL query that returns a book with corresponding input.
    ///
    /// # Example
    /// query {
    ///     getBooks(
    ///         filter: {format: HARDCOVER, author: "George Orwell"}
    ///     )
    ///     {
    ///         title
    ///         isbn
    ///     }
    /// }
    async fn get_books(&self, filter: FilteredBookInput) -> Vec<Book> {
        self.db.get_books(filter).await
    }

    /// # Example
    ///
    /// query {
    ///     getAllBooks {
    ///         isbn
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
    /// GraphQL mutation that adds multiple books to the database.
    /// Please note that some return value is required.
    ///
    /// # Example
    /// Please note that the title in the example below gets returned in order to see that things worked out properly
    ///
    /// mutation {
    ///     insertBook(
    ///         book: {isbn: "3", title: "1984", author: "George Orwell", format: HARDCOVER}
    ///     )
    ///     {
    ///	        title
    ///     }
    /// }
    /// # Errors
    /// Returns an error if any of the books couldn't be inserted properly
    async fn insert_book(&self, book: FilteredBookInput) -> Result<Book> {
        // Book is shadowed here since we want values to return aswell.
        let book = self.db.insert_book(book).await?;
        // Convert to book since that's the actual object
        Ok(book)
    }

    /// GraphQL mutation that adds a book to the database. `BookFormatType` was chosen here to avoid errors).
    /// Please note that some return value is required.
    ///
    /// # Example
    /// mutation {
    ///     insertBook(
    ///         books: [
    /// 	        { isbn: "1", title: "Book 1", author: "Author A", format: HARDCOVER },
    ///             { isbn: "2", title: "Book 2", author: "Author B", format: PAPERBACK },
    ///             { isbn: "3", title: "Book 3", author: "Author C", format: EPUB },
    ///         ]
    ///     )
    ///     {
    ///         title
    ///     }
    /// }
    /// # Errors
    /// Returns error if the book didn't insert properly
    async fn insert_books(&self, books: Vec<FilteredBookInput>) -> Result<Vec<Book>> {
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
