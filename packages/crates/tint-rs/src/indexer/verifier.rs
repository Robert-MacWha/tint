#[async_trait::async_trait]
pub trait Verifier {
    async fn verify(
        &self,
        to: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;
}
