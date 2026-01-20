use crate::book_schema::Book;
use crate::db::DB;
use async_graphql::Object;

pub(crate) struct Query {
    pub db: DB,
}

#[Object]
impl Query {
    // async fn get_book(&self, isbn: String) -> Option<Book> {
    // // Find book based on isbn
    // self.db
    // .get_mock_data()
    // .iter()
    // .find(|&x| x.isbn.0 == isbn)
    // .cloned()
    // }

    async fn get_all_books(&self) -> Vec<Book> {
        self.db.get_all_books().await
    }
}
