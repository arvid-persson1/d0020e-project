//!This crate implements a restful api
//!using the axum web framework which provides a
//!http based server for managing the books.
//! # Features
//! * Get a list of all books
//! * Get a book by isbn (id)
//! * Create a new book
///Module for handler functions
pub mod handlers;

#[cfg(test)]
mod api_testing;

use serde as _;
use std::error::Error;
use tokio::net::TcpListener;
use tokio::runtime::Builder;
pub mod app_builder;
use app_builder::build_app;

/// The application entry point.
///
/// # Errors
/// Returns an error if the Tokio runtime cannot be initialized,
/// if the TCP listener fails to bind, or if the server crashes.
fn main() -> Result<(), Box<dyn Error>> {
    let rt = Builder::new_multi_thread().enable_all().build()?;
    rt.block_on(async_main())
}

///The actual async function with the async logic
///
/// # Errors
///Returns an error if:
/// * The TCP listener fails to bind to the address (e.g., port 1616 is already in use).
/// * The Axum server fails to start or crashes during execution.
async fn async_main() -> Result<(), Box<dyn Error>> {
    //Create router for axum
    let app = build_app();

    //Define listener for axum (TCP: IP and port)
    let addrs = "127.0.0.1:1616";
    println!("App is listening on {addrs}");
    let tcplisn = TcpListener::bind(addrs).await?;

    //Call axum serve to start the web server
    axum::serve(tcplisn, app).await?;

    Ok(())
}
