//! Shared client helper for Yahoo Finance operations
//!
//! This module provides a cached YahooFinanceClient instance to avoid creating
//! new clients for every request and refreshing authentication unnecessarily.

use finance_query_core::{FetchClient, YahooAuthManager, YahooError, YahooFinanceClient};
use log::info;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::OnceLock;
use tokio::sync::Mutex;

static CLIENT: OnceLock<Arc<YahooFinanceClient>> = OnceLock::new();
static INIT_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Get or initialize the shared YahooFinanceClient instance
///
/// This function ensures the client is only initialized once, even under
/// concurrent access. Subsequent calls return the cached instance without
/// refreshing authentication.
///
/// The mutex ensures that even if multiple threads call this simultaneously
/// before initialization, only one will perform the auth refresh.
pub async fn get_yahoo_client() -> Result<Arc<YahooFinanceClient>, YahooError> {
    // Fast path: client already initialized (most common case)
    if let Some(client) = CLIENT.get() {
        return Ok(client.clone());
    }

    // Slow path: need to initialize (serialize concurrent attempts)
    // This mutex ensures only one thread performs initialization, even if
    // multiple threads pass the fast-path check simultaneously
    let _guard = INIT_MUTEX.lock().await;

    // Double-check after acquiring lock (another thread might have initialized it while we waited)
    if let Some(client) = CLIENT.get() {
        return Ok(client.clone());
    }

    // Initialize new client (only one thread will reach here, even under heavy concurrent load)
    info!("Initializing shared Yahoo Finance client (this should only happen once)");

    let fetch_client = Arc::new(FetchClient::new(None)?);
    let cookie_jar = fetch_client.cookie_jar().clone();
    let auth_manager = Arc::new(YahooAuthManager::new(None, cookie_jar));
    let client = Arc::new(YahooFinanceClient::new(auth_manager.clone(), fetch_client));

    // Refresh auth only once during initialization
    // This is the ONLY place where auth refresh should happen
    info!("Refreshing Yahoo authentication (one-time initialization)");
    auth_manager.refresh().await?;
    info!("Yahoo authentication refreshed successfully");

    // Store in OnceLock (should never fail since we checked above)
    CLIENT
        .set(client.clone())
        .map_err(|_| YahooError::ParseError("Failed to set client".to_string()))?;

    info!("Shared Yahoo Finance client initialized and cached");
    Ok(client)
}
