use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::sync::Mutex;

#[derive(Debug, Serialize, Deserialize, Clone)]
enum BookFormatType {
    Pdf,
    Word,
    Epub,
    Hardcover,
    Paperback,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Isbn(String);

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Book {
    id: Uuid,
    title: String,
    author: String,
    format: BookFormatType,
    isbn: Isbn,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateBook {
    title: String,
    author: String,
    format: BookFormatType,
    isbn: Isbn,
}

struct AppState {
    books: Mutex<Vec<Book>>,
}


async fn get_books(data: web::Data<AppState>) -> impl Responder {
    let books = (*data).books.lock().unwrap();
    HttpResponse::Ok().json(&*books)
}

async fn get_book(path: web::Path<Uuid>, data: web::Data<AppState>) -> impl Responder {
    let books = (*data).books.lock().unwrap();
    match books.iter().find(|b| b.id == *path) {
        Some(book) => HttpResponse::Ok().json(book),
        None => HttpResponse::NotFound().body("Book not found"),
    }
}

async fn create_book(book: web::Json<CreateBook>, data: web::Data<AppState>) -> impl Responder {
    let mut books = (*data).books.lock().unwrap();

    let new_book = Book {
        id: Uuid::new_v4(),
        title: (*book).title.clone(),
        author: (*book).author.clone(),
        format: (*book).format.clone(),
        isbn: (*book).isbn.clone(),
    };

    books.push(new_book);
    HttpResponse::Created().json(books.last().unwrap())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_state = web::Data::new(AppState {
        books: Mutex::new(vec![]),
    });

    println!("Bookstore API running at http://localhost:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(
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
