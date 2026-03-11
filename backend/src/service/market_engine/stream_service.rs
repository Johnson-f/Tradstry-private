//! Market Stream Service - Real-time market data streaming using finance-query-core
//!
//! Replaces the external WebSocket proxy with direct streaming from finance-query-core.
//! Manages subscriptions for quotes, indices, and movers, broadcasting updates to connected clients.

use anyhow::{Context, Result};
use dashmap::DashMap;
use futures_util::StreamExt;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use super::client_helper::get_yahoo_client;
use super::hours::get_hours;
use finance_query_core::{IndexStream, MoverCount, MoversStream, QuoteStream, YahooFinanceClient};

use crate::models::alerts::AlertType;
use crate::websocket::{ConnectionManager, EventType, WsMessage as AppWsMessage};
use crate::{
    service::notifications::{
        alert_worker::{PricePoint, evaluate_price_rules_for_user},
        alerts::AlertRuleService,
    },
    turso::AppState,
};
use chrono::Utc;

/// Quote update for frontend consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteUpdate {
    pub symbol: String,
    pub name: String,
    pub price: String,
    #[serde(rename = "preMarketPrice")]
    pub pre_market_price: Option<String>,
    #[serde(rename = "afterHoursPrice")]
    pub after_hours_price: Option<String>,
    pub change: String,
    #[serde(rename = "percentChange")]
    pub percent_change: String,
    pub logo: Option<String>,
}

/// Index update for frontend consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexUpdate {
    pub symbol: String,
    pub name: String,
    pub value: f64,
    pub change: String,
    #[serde(rename = "percentChange")]
    pub percent_change: String,
}

/// Mover item for frontend consumption
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoverItem {
    pub symbol: String,
    pub name: String,
    pub price: String,
    pub change: String,
    #[serde(rename = "percentChange")]
    pub percent_change: String,
}

/// Movers update containing gainers, losers, and most active
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoversUpdate {
    pub gainers: Vec<MoverItem>,
    pub losers: Vec<MoverItem>,
    pub actives: Vec<MoverItem>,
    pub timestamp: String,
}

/// Subscription type for different market data streams
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubscriptionType {
    Quote(String), // Individual stock quote
    Indices,       // Market indices (S&P 500, Dow, Nasdaq)
    Movers,        // Market movers (gainers, losers, actives)
}

/// Market Stream Service - manages real-time market data streaming
pub struct MarketStreamService {
    manager: Arc<Mutex<ConnectionManager>>,
    app_state: Arc<AppState>,
    client: Arc<Mutex<Option<Arc<YahooFinanceClient>>>>,
    /// Maps symbol -> Set of user_ids subscribed to quotes
    quote_subscriptions: Arc<DashMap<String, DashMap<String, bool>>>,
    /// Users subscribed to indices
    indices_subscribers: Arc<DashMap<String, bool>>,
    /// Users subscribed to movers
    movers_subscribers: Arc<DashMap<String, bool>>,
    /// Maps user_id -> Set of symbols they're subscribed to
    user_symbols: Arc<DashMap<String, DashMap<String, bool>>>,
    /// Active quote stream handle
    quote_stream_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Rule resync loop handle
    #[allow(dead_code)]
    rule_resync_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Active indices stream handle
    indices_stream_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Active movers stream handle
    movers_stream_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Channel to trigger quote stream restart with new symbols
    quote_restart_tx: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<Vec<String>>>>>,
    /// Polling interval in seconds
    poll_interval: u64,
    /// Rule resync interval in seconds
    #[allow(dead_code)]
    rule_resync_interval: u64,
}

impl MarketStreamService {
    /// Helper to determine if regular market hours are open.
    /// Falls back to `true` on errors to avoid unnecessarily blocking data.
    async fn market_is_open() -> bool {
        match get_hours().await {
            Ok(hours) => hours.reason.is_none(),
            Err(err) => {
                warn!("Market hours check failed, continuing stream: {}", err);
                true
            }
        }
    }

    /// Create a new MarketStreamService with shared app state
    pub fn new(
        manager: Arc<Mutex<ConnectionManager>>,
        poll_interval: u64,
        app_state: Arc<AppState>,
        rule_resync_interval: u64,
    ) -> Self {
        Self {
            manager,
            app_state,
            client: Arc::new(Mutex::new(None)),
            quote_subscriptions: Arc::new(DashMap::new()),
            indices_subscribers: Arc::new(DashMap::new()),
            movers_subscribers: Arc::new(DashMap::new()),
            user_symbols: Arc::new(DashMap::new()),
            quote_stream_handle: Arc::new(Mutex::new(None)),
            rule_resync_handle: Arc::new(Mutex::new(None)),
            indices_stream_handle: Arc::new(Mutex::new(None)),
            movers_stream_handle: Arc::new(Mutex::new(None)),
            quote_restart_tx: Arc::new(Mutex::new(None)),
            poll_interval,
            rule_resync_interval,
        }
    }

    /// Initialize the Yahoo Finance client (uses shared client)
    /// This ensures we use the globally shared client instance to avoid redundant auth refreshes
    async fn init_client(&self) -> Result<Arc<YahooFinanceClient>> {
        // Always use the shared client from client_helper
        // The client_helper handles singleton initialization and prevents concurrent auth refreshes
        let client = get_yahoo_client()
            .await
            .context("Failed to get Yahoo Finance client")?;

        // Cache it locally to avoid repeated lookups (but still use shared instance)
        let mut client_guard = self.client.lock().await;
        if client_guard.is_none() {
            *client_guard = Some(Arc::clone(&client));
            info!("Yahoo Finance client cached (using shared client instance)");
        }

        Ok(client)
    }

    /// Start all streaming services
    /// Note: Individual streams (indices, movers, quotes) are started on-demand when subscribers connect
    pub async fn start(&self) -> Result<()> {
        info!("Starting Market Stream Service");

        // Initialize client early to ensure it's ready before any streams start
        // This prevents concurrent auth refreshes when multiple streams start simultaneously
        let client = get_yahoo_client()
            .await
            .context("Failed to initialize Yahoo Finance client")?;

        // Cache it locally
        *self.client.lock().await = Some(client);

        info!(
            "Market Stream Service started successfully (shared client initialized, streams will start on-demand)"
        );
        Ok(())
    }

    #[allow(dead_code)]
    async fn start_rule_resync_loop(&self) {
        let interval = self.rule_resync_interval;
        if interval == 0 {
            return;
        }

        let this = self.clone_for_tasks();
        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(interval));
            loop {
                ticker.tick().await;
                if let Err(e) = this.sync_rule_subscriptions().await {
                    warn!("rule resync failed: {}", e);
                }
            }
        });

        let mut guard = self.rule_resync_handle.lock().await;
        *guard = Some(handle);
    }

    #[allow(dead_code)]
    fn clone_for_tasks(&self) -> Self {
        Self {
            manager: Arc::clone(&self.manager),
            app_state: Arc::clone(&self.app_state),
            client: Arc::clone(&self.client),
            quote_subscriptions: Arc::clone(&self.quote_subscriptions),
            indices_subscribers: Arc::clone(&self.indices_subscribers),
            movers_subscribers: Arc::clone(&self.movers_subscribers),
            user_symbols: Arc::clone(&self.user_symbols),
            quote_stream_handle: Arc::clone(&self.quote_stream_handle),
            rule_resync_handle: Arc::clone(&self.rule_resync_handle),
            indices_stream_handle: Arc::clone(&self.indices_stream_handle),
            movers_stream_handle: Arc::clone(&self.movers_stream_handle),
            quote_restart_tx: Arc::clone(&self.quote_restart_tx),
            poll_interval: self.poll_interval,
            rule_resync_interval: self.rule_resync_interval,
        }
    }

    /// Load all users' price alert rule symbols and ensure quote subscriptions are tracking them
    #[allow(dead_code)]
    async fn sync_rule_subscriptions(&self) -> Result<()> {
        // Disabled global sync; rule-based subscriptions are now loaded per user on-demand.
        Ok(())
    }

    /// Load a single user's price alert rule symbols and ensure quote subscriptions track them
    pub async fn sync_user_rule_subscriptions(&self, user_id: &str) -> Result<()> {
        let conn = match self
            .app_state
            .turso_client
            .get_user_database_connection(user_id)
            .await
        {
            Ok(Some(c)) => c,
            Ok(None) => {
                warn!(
                    "No database for user {} while syncing rule subscriptions",
                    user_id
                );
                return Ok(());
            }
            Err(e) => {
                warn!(
                    "DB connection error for user {} while syncing rule subscriptions: {}",
                    user_id, e
                );
                return Ok(());
            }
        };

        let service = AlertRuleService::new(&conn);
        let rules = match service.list_rules(user_id).await {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    "Failed to list alert rules for user {} while syncing: {}",
                    user_id, e
                );
                return Ok(());
            }
        };

        let mut added = false;
        for rule in rules
            .into_iter()
            .filter(|r| matches!(r.alert_type, AlertType::Price))
        {
            if let Some(sym) = rule.symbol.as_ref() {
                let sym_upper = sym.to_uppercase();
                let is_new_symbol = !self.quote_subscriptions.contains_key(&sym_upper);
                self.quote_subscriptions
                    .entry(sym_upper.clone())
                    .or_default()
                    .insert(user_id.to_string(), true);
                self.user_symbols
                    .entry(user_id.to_string())
                    .or_default()
                    .insert(sym_upper.clone(), true);
                if is_new_symbol {
                    added = true;
                }
            }
        }

        if added {
            self.trigger_quote_stream_update().await;
        }

        Ok(())
    }

    /// Start the indices stream (only if not already running)
    async fn start_indices_stream(&self) -> Result<()> {
        // Check if stream is already running
        {
            let handle_guard = self.indices_stream_handle.lock().await;
            if handle_guard.is_some() {
                return Ok(()); // Already running
            }
        }

        // Skip starting when market is closed
        if !Self::market_is_open().await {
            info!("Market closed, skipping indices stream start");
            return Ok(());
        }

        let client = self.init_client().await?;
        let manager = Arc::clone(&self.manager);
        let subscribers = Arc::clone(&self.indices_subscribers);
        let poll_interval = self.poll_interval;

        let handle = tokio::spawn(async move {
            let indices = vec![
                "^GSPC".to_string(), // S&P 500
                "^DJI".to_string(),  // Dow Jones
                "^IXIC".to_string(), // Nasdaq
                "^RUT".to_string(),  // Russell 2000
            ];

            let mut stream =
                IndexStream::create(client, indices, Duration::from_secs(poll_interval));

            info!("Indices stream started with {}s interval", poll_interval);

            while let Some(result) = stream.next().await {
                if !MarketStreamService::market_is_open().await {
                    info!("Market closed, stopping indices stream");
                    break;
                }

                // Check if we have any subscribers before fetching
                if subscribers.is_empty() {
                    warn!("No indices subscribers, stopping stream");
                    break;
                }

                match result {
                    Ok(indices_data) => {
                        // Double-check subscribers before broadcasting
                        if subscribers.is_empty() {
                            break;
                        }

                        let updates: Vec<IndexUpdate> = indices_data
                            .iter()
                            .map(|idx| IndexUpdate {
                                symbol: idx.name.clone(), // Use name as symbol since MarketIndex doesn't have symbol field
                                name: idx.name.clone(),
                                value: idx.value,
                                change: idx.change.clone(),
                                percent_change: idx.percent_change.clone(),
                            })
                            .collect();

                        // Broadcast to all subscribers
                        let manager = manager.lock().await;
                        let message = AppWsMessage::new(
                            EventType::MarketUpdate,
                            serde_json::json!({
                                "type": "indices",
                                "data": updates
                            }),
                        );

                        // Broadcast to all subscribed users
                        for entry in subscribers.iter() {
                            manager.broadcast_to_user(entry.key(), message.clone());
                        }
                    }
                    Err(e) => {
                        error!("Error fetching indices: {}", e);
                    }
                }
            }

            info!("Indices stream stopped");
        });

        *self.indices_stream_handle.lock().await = Some(handle);
        Ok(())
    }

    /// Start the movers stream (only if not already running)
    async fn start_movers_stream(&self) -> Result<()> {
        // Check if stream is already running
        {
            let handle_guard = self.movers_stream_handle.lock().await;
            if handle_guard.is_some() {
                return Ok(()); // Already running
            }
        }

        // Skip starting when market is closed
        if !Self::market_is_open().await {
            info!("Market closed, skipping movers stream start");
            return Ok(());
        }

        let client = self.init_client().await?;
        let manager = Arc::clone(&self.manager);
        let subscribers = Arc::clone(&self.movers_subscribers);
        let poll_interval = self.poll_interval;

        let handle = tokio::spawn(async move {
            let mut stream = MoversStream::create(
                client,
                MoverCount::TwentyFive,
                Duration::from_secs(poll_interval),
            );

            info!("Movers stream started with {}s interval", poll_interval);

            while let Some(result) = stream.next().await {
                if !MarketStreamService::market_is_open().await {
                    info!("Market closed, stopping movers stream");
                    break;
                }

                // Check if we have any subscribers before fetching
                if subscribers.is_empty() {
                    warn!("No movers subscribers, stopping stream");
                    break;
                }

                match result {
                    Ok(movers_data) => {
                        // Double-check subscribers before broadcasting
                        if subscribers.is_empty() {
                            break;
                        }

                        let update = MoversUpdate {
                            gainers: movers_data
                                .gainers
                                .iter()
                                .take(10)
                                .map(|m| MoverItem {
                                    symbol: m.symbol.clone(),
                                    name: m.name.clone(),
                                    price: m.price.clone(),
                                    change: m.change.clone(),
                                    percent_change: m.percent_change.clone(),
                                })
                                .collect(),
                            losers: movers_data
                                .losers
                                .iter()
                                .take(10)
                                .map(|m| MoverItem {
                                    symbol: m.symbol.clone(),
                                    name: m.name.clone(),
                                    price: m.price.clone(),
                                    change: m.change.clone(),
                                    percent_change: m.percent_change.clone(),
                                })
                                .collect(),
                            actives: movers_data
                                .actives
                                .iter()
                                .take(10)
                                .map(|m| MoverItem {
                                    symbol: m.symbol.clone(),
                                    name: m.name.clone(),
                                    price: m.price.clone(),
                                    change: m.change.clone(),
                                    percent_change: m.percent_change.clone(),
                                })
                                .collect(),
                            timestamp: movers_data.timestamp.format("%H:%M:%S").to_string(),
                        };

                        // Broadcast to all subscribers
                        let manager = manager.lock().await;
                        let message = AppWsMessage::new(
                            EventType::MarketUpdate,
                            serde_json::json!({
                                "type": "movers",
                                "data": update
                            }),
                        );

                        for entry in subscribers.iter() {
                            manager.broadcast_to_user(entry.key(), message.clone());
                        }
                    }
                    Err(e) => {
                        error!("Error fetching movers: {}", e);
                    }
                }
            }

            info!("Movers stream stopped");
        });

        *self.movers_stream_handle.lock().await = Some(handle);
        Ok(())
    }

    /// Start or restart the quote stream with current subscribed symbols
    async fn start_quote_stream(&self) -> Result<()> {
        // Cancel existing stream if any
        if let Some(handle) = self.quote_stream_handle.lock().await.take() {
            handle.abort();
        }

        let symbols: Vec<String> = self
            .quote_subscriptions
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        if symbols.is_empty() {
            info!("No symbols to stream, quote stream not started");
            return Ok(());
        }

        // Skip starting when market is closed
        if !Self::market_is_open().await {
            info!("Market closed, skipping quote stream start");
            return Ok(());
        }

        let client = self.init_client().await?;
        let manager = Arc::clone(&self.manager);
        let subscriptions = Arc::clone(&self.quote_subscriptions);
        let app_state = Arc::clone(&self.app_state);
        let poll_interval = self.poll_interval;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<String>>();

        *self.quote_restart_tx.lock().await = Some(tx);

        let handle = tokio::spawn(async move {
            let mut current_symbols = symbols;

            loop {
                if !MarketStreamService::market_is_open().await {
                    info!("Market closed, stopping quote stream");
                    return;
                }

                info!(
                    "Starting quote stream for {} symbols",
                    current_symbols.len()
                );

                let mut stream = QuoteStream::create(
                    Arc::clone(&client),
                    current_symbols.clone(),
                    Duration::from_secs(poll_interval),
                );

                loop {
                    tokio::select! {
                        // Check for symbol updates
                        new_symbols = rx.recv() => {
                            match new_symbols {
                                Some(symbols) if symbols != current_symbols => {
                                    info!("Restarting quote stream with {} symbols", symbols.len());
                                    current_symbols = symbols;
                                    break; // Break inner loop to restart stream
                                }
                                Some(_) => continue, // Same symbols, ignore
                                None => {
                                    info!("Quote stream channel closed");
                                    return;
                                }
                            }
                        }
                        // Process quote updates
                        result = stream.next() => {
                            // Check if we have any subscribers before processing
                            if subscriptions.is_empty() {
                                warn!("No quote subscribers, stopping stream");
                                break;
                            }

                            match result {
                                Some(Ok(update)) => {
                                    if !MarketStreamService::market_is_open().await {
                                        info!("Market closed, stopping quote stream");
                                        return;
                                    }

                                    // Double-check subscribers before broadcasting
                                    if subscriptions.is_empty() {
                                        break;
                                    }

                                    let manager = manager.lock().await;

                                    for quote in &update.quotes {
                                        let quote_update = QuoteUpdate {
                                            symbol: quote.symbol.clone(),
                                            name: quote.name.clone(),
                                            price: quote.price.clone(),
                                            pre_market_price: quote.pre_market_price.clone(),
                                            after_hours_price: quote.after_hours_price.clone(),
                                            change: quote.change.clone(),
                                            percent_change: quote.percent_change.clone(),
                                            logo: None,
                                        };

                                        // Broadcast to users subscribed to this symbol
                                        if let Some(user_set) = subscriptions.get(&quote.symbol) {
                                            let message = AppWsMessage::new(
                                                EventType::MarketQuote,
                                                serde_json::to_value(&quote_update).unwrap_or_default(),
                                            );

                                            for entry in user_set.iter() {
                                                manager.broadcast_to_user(entry.key(), message.clone());
                                            }
                                        }

                                        // Evaluate price alerts for subscribers of this symbol
                                        if let Ok(price_f64) = quote.price.parse::<f64>()
                                            && let Some(user_set) = subscriptions.get(&quote.symbol)
                                        {
                                            let subs: Vec<String> = user_set.iter().map(|e| e.key().clone()).collect();
                                            let app_state = Arc::clone(&app_state);
                                            let symbol = quote.symbol.clone();
                                            let percent_change = {
                                                let cleaned = quote.percent_change.replace(['%', '+'], "");
                                                cleaned.parse::<f64>().ok()
                                            };

                                            tokio::spawn(async move {
                                                for uid in subs {
                                                    match app_state
                                                        .turso_client
                                                        .get_user_database_connection(&uid)
                                                        .await
                                                    {
                                                        Ok(Some(conn)) => {
                                                            let point = PricePoint {
                                                                symbol: symbol.clone(),
                                                                price: price_f64,
                                                                direction: None,
                                                                percent_change,
                                                                timestamp: Utc::now(),
                                                            };
                                                            if let Err(e) = evaluate_price_rules_for_user(
                                                                &conn,
                                                                &app_state.config.web_push,
                                                                &uid,
                                                                &[point],
                                                            )
                                                            .await
                                                            {
                                                                error!(
                                                                    "Failed to evaluate price alerts for user {}: {}",
                                                                    uid, e
                                                                );
                                                            }
                                                        }
                                                        Ok(None) => {
                                                            warn!(
                                                                "No database found for user {} while evaluating alerts",
                                                                uid
                                                            );
                                                        }
                                                        Err(e) => {
                                                            error!(
                                                                "DB connection error for user {} while evaluating alerts: {}",
                                                                uid, e
                                                            );
                                                        }
                                                    }
                                                }
                                            });
                                        }
                                    }
                                }
                                Some(Err(e)) => {
                                    error!("Error fetching quotes: {}", e);
                                }
                                None => {
                                    warn!("Quote stream ended unexpectedly");
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });

        *self.quote_stream_handle.lock().await = Some(handle);
        Ok(())
    }

    /// Subscribe a user to quote updates for symbols
    pub async fn subscribe_quotes(&self, user_id: &str, symbols: &[String]) -> Result<()> {
        // Ensure this user's price-rule symbols are tracked without touching all users
        if let Err(e) = self.sync_user_rule_subscriptions(user_id).await {
            warn!("sync_user_rule_subscriptions failed for {}: {}", user_id, e);
        }

        let mut needs_restart = false;

        for symbol in symbols {
            let sym_upper = symbol.to_uppercase();

            // Check if this is a new symbol
            let is_new = !self.quote_subscriptions.contains_key(&sym_upper);

            self.quote_subscriptions
                .entry(sym_upper.clone())
                .or_default()
                .insert(user_id.to_string(), true);

            self.user_symbols
                .entry(user_id.to_string())
                .or_default()
                .insert(sym_upper.clone(), true);

            if is_new {
                needs_restart = true;
            }

            info!("User {} subscribed to quote: {}", user_id, sym_upper);
        }

        // Restart stream if new symbols added
        if needs_restart {
            self.trigger_quote_stream_update().await;
        }

        Ok(())
    }

    /// Unsubscribe a user from quote updates for symbols
    pub async fn unsubscribe_quotes(&self, user_id: &str, symbols: &[String]) -> Result<()> {
        let mut needs_restart = false;

        for symbol in symbols {
            let sym_upper = symbol.to_uppercase();

            if let Some(user_set) = self.quote_subscriptions.get_mut(&sym_upper) {
                user_set.remove(user_id);
                if user_set.is_empty() {
                    drop(user_set);
                    self.quote_subscriptions.remove(&sym_upper);
                    needs_restart = true;
                }
            }

            if let Some(symbol_set) = self.user_symbols.get_mut(user_id) {
                symbol_set.remove(&sym_upper);
            }

            info!("User {} unsubscribed from quote: {}", user_id, sym_upper);
        }

        // Check if we have any symbols left to stream
        if self.quote_subscriptions.is_empty() {
            // Stop the quote stream if no symbols remain
            if let Some(handle) = self.quote_stream_handle.lock().await.take() {
                handle.abort();
                info!("Stopped quote stream (no symbols subscribed)");
            }
            // Clear the restart channel
            *self.quote_restart_tx.lock().await = None;
        } else if needs_restart {
            // Restart with remaining symbols
            self.trigger_quote_stream_update().await;
        }

        Ok(())
    }

    /// Subscribe a user to indices updates
    pub async fn subscribe_indices(&self, user_id: &str) -> Result<()> {
        let is_first_subscriber = self.indices_subscribers.is_empty();
        self.indices_subscribers.insert(user_id.to_string(), true);
        info!("User {} subscribed to indices", user_id);

        // Start stream if this is the first subscriber
        if is_first_subscriber && let Err(e) = self.start_indices_stream().await {
            error!("Failed to start indices stream: {}", e);
            // Remove the subscriber if stream failed to start
            self.indices_subscribers.remove(user_id);
            return Err(e);
        }

        Ok(())
    }

    /// Unsubscribe a user from indices updates
    pub async fn unsubscribe_indices(&self, user_id: &str) -> Result<()> {
        self.indices_subscribers.remove(user_id);
        info!("User {} unsubscribed from indices", user_id);

        // Stop stream if no more subscribers
        if self.indices_subscribers.is_empty()
            && let Some(handle) = self.indices_stream_handle.lock().await.take()
        {
            handle.abort();
            info!("Stopped indices stream (no subscribers)");
        }

        Ok(())
    }

    /// Subscribe a user to movers updates
    pub async fn subscribe_movers(&self, user_id: &str) -> Result<()> {
        let is_first_subscriber = self.movers_subscribers.is_empty();
        self.movers_subscribers.insert(user_id.to_string(), true);
        info!("User {} subscribed to movers", user_id);

        // Start stream if this is the first subscriber
        if is_first_subscriber && let Err(e) = self.start_movers_stream().await {
            error!("Failed to start movers stream: {}", e);
            // Remove the subscriber if stream failed to start
            self.movers_subscribers.remove(user_id);
            return Err(e);
        }

        Ok(())
    }

    /// Unsubscribe a user from movers updates
    pub async fn unsubscribe_movers(&self, user_id: &str) -> Result<()> {
        self.movers_subscribers.remove(user_id);
        info!("User {} unsubscribed from movers", user_id);

        // Stop stream if no more subscribers
        if self.movers_subscribers.is_empty()
            && let Some(handle) = self.movers_stream_handle.lock().await.take()
        {
            handle.abort();
            info!("Stopped movers stream (no subscribers)");
        }

        Ok(())
    }

    /// Trigger quote stream update with debouncing
    async fn trigger_quote_stream_update(&self) {
        let symbols: Vec<String> = self
            .quote_subscriptions
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        if let Some(tx) = self.quote_restart_tx.lock().await.as_ref() {
            let _ = tx.send(symbols);
        } else {
            // Stream not started yet, start it
            if let Err(e) = self.start_quote_stream().await {
                error!("Failed to start quote stream: {}", e);
            }
        }
    }

    /// Get all symbols a user is subscribed to
    pub fn get_user_subscriptions(&self, user_id: &str) -> Vec<String> {
        self.user_symbols
            .get(user_id)
            .map(|entry| entry.iter().map(|e| e.key().clone()).collect())
            .unwrap_or_default()
    }

    /// Unsubscribe user from all streams (called on disconnect)
    pub async fn unsubscribe_all(&self, user_id: &str) -> Result<()> {
        // Unsubscribe from all quotes
        let symbols = self.get_user_subscriptions(user_id);
        if !symbols.is_empty() {
            self.unsubscribe_quotes(user_id, &symbols).await?;
        }

        // Unsubscribe from indices and movers
        self.unsubscribe_indices(user_id).await?;
        self.unsubscribe_movers(user_id).await?;

        // Clean up user_symbols entry
        self.user_symbols.remove(user_id);

        info!("User {} unsubscribed from all streams", user_id);
        Ok(())
    }
}
