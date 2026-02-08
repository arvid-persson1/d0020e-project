//! Compile-time tests for broker query macros.

// Silence unused-crate-dependencies lint for compile-test crate
use broker as _;
use bytes as _;
use either as _;
use futures as _;
use nameof as _;
use query_macro as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use thiserror as _;
use tokio as _;
use transitive as _;
use trybuild as _;

use broker::query::*;

#[cfg(test)]
mod tests {
    use super::*;
    use trybuild as _;

    /// Book format variants.
    #[derive(Debug, PartialEq)]
    enum BookFormatType {
        Hardcover,
        Paperback,
        Ebook,
    }

    /// Simple ISBN wrapper.
    #[derive(Debug, PartialEq, Queryable)]
    struct Isbn {
        value: String,
    }

    #[derive(Queryable)]
    struct Book {
        title: String,
        author: String,
        format: BookFormatType,
        isbn: Isbn,
    }

    /// # Panics
    ///
    /// Panics if the query evaluation fails.
    #[test]
    fn simple_field_eq() {
        let title = "The Rust Programming Language".to_owned();
        let q = Book::title().eq(&title);

        let book = Book {
            title: "The Rust Programming Language".into(),
            author: "Steve Klabnik & Carol Nichols".into(),
            format: BookFormatType::Hardcover,
            isbn: Isbn {
                value: "978-1593278281".into(),
            },
        };

        assert!(q.evaluate(&book));
    }

    /// # Panics
    ///
    /// Panics if the query evaluation fails.
    #[test]
    fn enum_field_eq() {
        let q = Book::format().eq(&BookFormatType::Ebook);

        let book = Book {
            title: "Rust for Professionals".into(),
            author: "Jane Doe".into(),
            format: BookFormatType::Ebook,
            isbn: Isbn {
                value: "978-0000000000".into(),
            },
        };

        assert!(q.evaluate(&book));
    }

    /// # Panics
    ///
    /// Panics if the query evaluation fails.
    #[test]
    fn nested_field_eq() {
        let q = Book::isbn().then(&Isbn::value()).eq("978-1593278281");

        let book = Book {
            title: "The Rust Programming Language".into(),
            author: "Steve Klabnik & Carol Nichols".into(),
            format: BookFormatType::Paperback,
            isbn: Isbn {
                value: "978-1593278281".into(),
            },
        };

        assert!(q.evaluate(&book));
    }

    /// # Panics
    ///
    /// Panics if the query evaluation fails.
    #[test]
    fn author_eq() {
        let q = Book::author().eq("Steve Klabnik & Carol Nichols");

        let book = Book {
            title: "The Rust Programming Language".into(),
            author: "Steve Klabnik & Carol Nichols".into(),
            format: BookFormatType::Paperback,
            isbn: Isbn {
                value: "978-1593278281".into(),
            },
        };

        assert!(q.evaluate(&book));
    }
}
