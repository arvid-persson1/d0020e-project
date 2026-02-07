// @generated automatically by Diesel CLI.
/// Module created by the diesel.rs ORM for the sql types used.
pub mod sql_types {
    #[derive(Debug, diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "book_format_type"))]
    /// Struct for the type of books format
    pub struct BookFormatType;
}

diesel::table! {
    use diesel::sql_types::VarChar;
    use super::sql_types::BookFormatType;
    /// Struct for the books schema created by the ORM
    books (isbn) {
        /// Title attribute
        title -> VarChar,
        /// Author attribute
        author -> VarChar,
        /// Format attribute
        format -> BookFormatType,
        /// Isbn attribute. is set to be the id.
        isbn -> VarChar,
    }
}
