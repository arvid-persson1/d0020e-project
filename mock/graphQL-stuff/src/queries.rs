use crate::book_schema::Book;
use crate::db::DB;
use async_graphql::Object;

pub(crate) struct Query {
    pub db: DB,
}

#[Object]
impl Query {
    async fn get_book(&self, id: String) -> Option<Book> {
        // This feels illegal, but all it does is return the element with the correpsonding id
        self.db
            .get_mock_data()
            .iter()
            .find(|&x| x.id.0 == id)
            .cloned()
    }

    async fn get_all_books(&self) -> Vec<Book> {
        self.db.get_mock_data()
    }
}
