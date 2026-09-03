#[tokio::main]
async fn main() {
    mousuo::db::init().await;
    mousuo::db::schema::ensure_tables()
        .await
        .unwrap_or_else(|e| panic!("ensure tables: {e}"));
    mousuo::db::products::seed_products()
        .await
        .unwrap_or_else(|e| panic!("seed products: {e}"));

    topcoat::start(mousuo::app::router()).await.unwrap();
}
