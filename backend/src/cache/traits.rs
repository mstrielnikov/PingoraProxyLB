use async_trait::async_trait;

#[async_trait]
pub trait CacheTrait {
    async fn get(&self, key: &str) -> Option<String>;
    async fn insert(&self, key: String, value: String);
    async fn evict(&self, key: &str);
}
