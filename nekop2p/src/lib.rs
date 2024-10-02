#[tarpc::service]
pub trait Indexer {
    async fn register(filename: String);
    async fn search(filename: String) -> Vec<String>;
    async fn deregister(filename: String);
}
