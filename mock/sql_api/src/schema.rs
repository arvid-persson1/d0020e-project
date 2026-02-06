// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(Debug, diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "book_format_type"))]
    pub struct BookFormatType;
}

diesel::table! {
    use diesel::sql_types::VarChar;
    use super::sql_types::BookFormatType;

    books (isbn) {
        title -> VarChar,
        author -> VarChar,
        format -> BookFormatType,
        isbn -> VarChar,
    }
}
