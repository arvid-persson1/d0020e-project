//! A simple Actix Web API for managing books in memory.
//! This crate provides endpoints to list, create, and fetch books.
use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use serde::{Deserialize, Serialize};
use std::io::Result;
//use std::sync::Mutex;
use parking_lot::Mutex;
use uuid::Uuid;

/// Available book formats
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
enum BookFormatType {
    /// pdf format
    Pdf,
    /// Microsoft word format
    Word,
    /// EPUB format
    Epub,
    /// Hardcover printed out format
    Hardcover,
    /// Paperback printed out format
    Paperback,
    /// Pocket edition
    Pocket,
}

/// ISBN identifier
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Isbn(String);

/// Stored book
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Book {
    /// Unique book identifier
    id: Uuid,
    /// Book title
    title: String,
    /// Book author
    author: String,
    /// Book format
    format: BookFormatType,
    /// Book ISBN
    isbn: Isbn,
}

/// Make new book
#[derive(Debug, Serialize, Deserialize)]
struct CreateBook {
    /// New book title
    title: String,
    /// New book author
    author: String,
    /// New book format
    format: BookFormatType,
    /// New book ISBN
    isbn: Isbn,
}

/// Shared application state for the Bookstore API.
/// Holds an in-memory list of books wrapped in a mutex
struct AppState {
    /// Thread-safe in-memory storage of books.
    books: Mutex<Vec<Book>>,
}

/// Query parameters for searching books.
///
/// All fields are optional. When a field is present, it is used to
/// filter the returned books. Multiple fields are combined using
/// logical AND semantics.
#[derive(Deserialize)]
struct BookSearch {
    /// Filter books by exact title match.
    ///
    /// Example:
    /// `GET /books?title=Moby%20Dick`
    title: Option<String>,
    /// Filter books by exact author name.
    ///
    /// Example:
    /// `GET /books?author=Herman%20Melville`
    author: Option<String>,
    /// Filter books by book format.
    ///
    /// Example:
    /// `GET /books?format=Pdf`
    format: Option<BookFormatType>,
}

/// Returns all books currently stored.
///
/// # Panics
/// Panics if acquiring the books mutex fails.
async fn get_books(query: web::Query<BookSearch>, data: web::Data<AppState>) -> impl Responder {
    let results: Vec<Book> = {
        let books = data.books.lock();

        books
            .iter()
            .filter(|b| {
                query.title.as_ref().is_none_or(|t| b.title == *t)
                    && query.author.as_ref().is_none_or(|a| b.author == *a)
                    && query.format.as_ref().is_none_or(|f| b.format == *f)
            })
            .cloned()
            .collect()
    };

    HttpResponse::Ok().json(results)
}

/// Returns specific book
///
/// # Panics
/// Panics if locking the mutex fails.
async fn get_book(path: web::Path<Uuid>, data: web::Data<AppState>) -> impl Responder {
    let books = data.books.lock();

    books.iter().find(|b| b.id == *path).map_or_else(
        || HttpResponse::NotFound().body("Book not found"),
        |book| HttpResponse::Ok().json(book),
    )
}

/// Creates book
///
/// # Panics
/// Panics if locking the mutex fails.
async fn create_book(book: web::Json<CreateBook>, data: web::Data<AppState>) -> impl Responder {
    let mut books = data.books.lock();

    let new_book = Book {
        id: Uuid::new_v4(),
        title: book.title.clone(),
        author: book.author.clone(),
        format: book.format.clone(),
        isbn: book.isbn.clone(),
    };

    books.push(new_book);
    books.last().map_or_else(
        || HttpResponse::InternalServerError().body("Book creation failed"),
        |created| HttpResponse::Created().json(created),
    )
}

#[actix_web::main]
async fn main() -> Result<()> {
    let app_state = web::Data::new(AppState {
        books: Mutex::new(vec![
            Book {
                id: Uuid::new_v4(),
                title: "Pride and Prejudice".to_owned(),
                author: "Jane Austen".to_owned(),
                format: BookFormatType::Paperback,
                isbn: Isbn("9780141439518".to_owned()),
            },
            Book {
                id: Uuid::new_v4(),
                title: "Moby Dick".to_owned(),
                author: "Herman Melville".to_owned(),
                format: BookFormatType::Hardcover,
                isbn: Isbn("9781503280786".to_owned()),
            },
            Book {
                id: Uuid::new_v4(),
                title: "Animal Farm".to_owned(),
                author: "George Orwell".to_owned(),
                format: BookFormatType::Paperback,
                isbn: Isbn("9780141036137".to_owned()),
            },
            Book {
                id: Uuid::new_v4(),
                title: "1984".to_owned(),
                author: "George Orwell".to_owned(),
                format: BookFormatType::Paperback,
                isbn: Isbn("9780141036144".to_owned()),
            },
        ]),
    });

    println!("Bookstore API running at http://localhost:8080");

    HttpServer::new(move || {
        App::new().app_data(app_state.clone()).service(
            web::scope("/books")
                .route("", web::get().to(get_books))
                .route("", web::post().to(create_book))
                .route("/{id}", web::get().to(get_book)),
        )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
