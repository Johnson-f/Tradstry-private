use clerk_rs::{
    clerk::Clerk,
    validators::jwks::MemoryCacheJwksProvider,
    ClerkConfiguration,
};

pub fn create_jwks_provider(secret_key: &str) -> MemoryCacheJwksProvider {
    let config = ClerkConfiguration::new(None, None, Some(secret_key.to_string()), None);
    let clerk = Clerk::new(config);
    MemoryCacheJwksProvider::new(clerk)
}
