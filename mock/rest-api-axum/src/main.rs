mod handlers;

use std::sync::{Arc, Mutex};

use axum::{
  Router,
  routing::{get, post},
};

use handlers::{APPState, get_books, get_book, add_book};

#[tokio::main]
async fn main(){

  let books = vec![];

  let state = Arc::new(APPState{
    books: Arc::new(Mutex::new(books)),
  });

  //Create router for axum
  let app = Router::new()
    .route("/books", get(get_books))
    .route("/books", get(get_book))
    .route("/books", post(add_book)).with_state(state);

  //Define listener for axum (TCP: IP and port)
  let addrs = "127.0.0.1:1616";
  println!("App is listening on {}", addrs);
  let tcplisn = tokio::net::TcpListener::bind(addrs).await.unwrap();

  //Call axum serve to start the web server
  axum::serve(tcplisn, app).await.unwrap();
}
