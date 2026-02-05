pub mod db;
pub mod models;
pub mod schema;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use serde::Serialize;

fn main() {
    println!("Hello, world!");
}
