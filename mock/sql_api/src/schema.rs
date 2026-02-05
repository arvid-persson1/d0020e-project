// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "book_format_type"))]
    pub struct BookFormatType;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::BookFormatType;

    books (isbn) {
        title -> Varchar,
        author -> Varchar,
        format -> BookFormatType,
        isbn -> Varchar,
    }
}
