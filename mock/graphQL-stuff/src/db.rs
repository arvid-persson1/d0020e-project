use crate::book_schema::{Book, BookFormatType};
use async_graphql::ID;

pub(crate) struct DB;

// Pretty much just a proof of concept for fetching.
// Remeber to make it a real database later.
impl DB {
    pub(crate) fn get_mock_data(&self) -> Vec<Book> {
        vec![
            Book {
                id: ID::from("1"),
                title: "The Rust Programming Language".to_string(),
                author: "Steve Klabnik and Carol Nichols".to_string(),
                isbn: "978-1593278281".to_string(),
                format: BookFormatType::Pdf,
            },
            Book {
                id: ID::from("2"),
                title: "Cracking the Coding Interview".to_string(),
                author: "Gayle Laakmann McDowell".to_string(),
                isbn: "978-0984782857".to_string(),
                format: BookFormatType::Paperback,
            },
            Book {
                id: ID::from("3"),
                title: "Designing Data-Intensive Applications".to_string(),
                author: "Martin Kleppmann".to_string(),
                isbn: "978-1449373320".to_string(),
                format: BookFormatType::Epub,
            },
            Book {
                id: ID::from("4"),
                title: "The Hitchhiker's Guide to the Galaxy".to_string(),
                author: "Douglas Adams".to_string(),
                isbn: "978-0345391803".to_string(),
                format: BookFormatType::Hardcover,
            },
            Book {
                id: ID::from("5"),
                title: "Clean Code: A Handbook of Agile Software Craftsmanship".to_string(),
                author: "Robert C. Martin".to_string(),
                isbn: "978-0132350884".to_string(),
                format: BookFormatType::Word,
            },
        ]
    }
}
