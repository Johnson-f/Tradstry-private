use anyhow::Result;
use libsql::{Connection, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use web_push::{
    ContentEncoding, SubscriptionInfo, VapidSignatureBuilder, WebPushClient, WebPushMessageBuilder,
};

use crate::turso::config::WebPushConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushSubscription {
    pub id: String,
    pub user_id: String,
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub ua: Option<String>,
    pub topics: Option<String>, // JSON array string
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSubscriptionRequest {
    pub endpoint: String,
    pub keys: PushKeys,
    pub ua: Option<String>,
    pub topics: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushKeys {
    pub p256dh: String,
    pub auth: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushPayload {
    pub title: String,
    pub body: Option<String>,
    pub icon: Option<String>,
    pub url: Option<String>,
    pub tag: Option<String>,
    pub data: Option<serde_json::Value>,
}

pub struct PushService<'a> {
    pub conn: &'a Connection,
    pub web_push: &'a WebPushConfig,
}

impl<'a> PushService<'a> {
    pub fn new(conn: &'a Connection, web_push: &'a WebPushConfig) -> Self {
        Self { conn, web_push }
    }

    pub async fn upsert_subscription(
        &self,
        user_id: &str,
        req: SaveSubscriptionRequest,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let topics_json = req
            .topics
            .map(|t| serde_json::to_string(&t).unwrap_or_else(|_| "[]".to_string()));
        let now = chrono::Utc::now().to_rfc3339();
        let endpoint = req.endpoint.clone();
        let p256dh = req.keys.p256dh.clone();
        let auth = req.keys.auth.clone();
        let ua = req.ua.clone().unwrap_or_default();

        // Try update by endpoint; if 0 rows, insert
        let updated = self.conn.execute(
            "UPDATE push_subscriptions SET user_id = ?, p256dh = ?, auth = ?, ua = ?, topics = ?, updated_at = ? WHERE endpoint = ?",
            params![user_id, p256dh.clone(), auth.clone(), ua.clone(), topics_json.clone().unwrap_or_default(), now.clone(), endpoint.clone()],
        ).await?;

        if updated == 0 {
            self.conn.execute(
                "INSERT INTO push_subscriptions (id, user_id, endpoint, p256dh, auth, ua, topics, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![id.clone(), user_id, endpoint, p256dh, auth, ua, topics_json.unwrap_or_default(), now.clone(), now],
            ).await?;
            Ok(id)
        } else {
            // Return existing id if needed; not critical here
            Ok(id)
        }
    }

    pub async fn remove_subscription(&self, user_id: &str, endpoint: &str) -> Result<bool> {
        let n = self
            .conn
            .execute(
                "DELETE FROM push_subscriptions WHERE user_id = ? AND endpoint = ?",
                params![user_id, endpoint],
            )
            .await?;
        Ok(n > 0)
    }

    pub async fn list_user_subscriptions(&self, user_id: &str) -> Result<Vec<PushSubscription>> {
        let mut rows = self.conn
            .prepare("SELECT id, user_id, endpoint, p256dh, auth, ua, topics, created_at, updated_at FROM push_subscriptions WHERE user_id = ?")
            .await?
            .query(params![user_id])
            .await?;

        let mut items = Vec::new();
        while let Some(row) = rows.next().await? {
            items.push(PushSubscription {
                id: row.get(0)?,
                user_id: row.get(1)?,
                endpoint: row.get(2)?,
                p256dh: row.get(3)?,
                auth: row.get(4)?,
                ua: row.get(5)?,
                topics: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            });
        }
        Ok(items)
    }

    pub async fn send_to_user(&self, user_id: &str, payload: &PushPayload) -> Result<()> {
        let subs = self.list_user_subscriptions(user_id).await?;
        if subs.is_empty() {
            return Ok(());
        }

        // Create the web push client - use IsahcWebPushClient (default) or HyperWebPushClient
        let client = web_push::IsahcWebPushClient::new()?;
        let body = serde_json::to_vec(payload)?;

        for sub in subs {
            let info = SubscriptionInfo::new(&sub.endpoint, &sub.p256dh, &sub.auth);

            // Retry up to 3 times on transient errors
            let mut attempts = 0;
            let mut backoff_ms = 200u64;

            loop {
                attempts += 1;

                // Build the message
                let mut builder = WebPushMessageBuilder::new(&info);
                builder.set_payload(ContentEncoding::Aes128Gcm, &body);

                // Create VAPID signature - in 0.11.0, from_base64 takes subscription info
                // and build() is called on the builder to get the signature
                let mut sig_builder =
                    VapidSignatureBuilder::from_base64(&self.web_push.vapid_private_key, &info)?;

                // Add the subject claim (e.g., "mailto:admin@example.com")
                sig_builder.add_claim("sub", self.web_push.subject.as_str());

                // Build the signature
                let signature = sig_builder.build()?;
                builder.set_vapid_signature(signature);

                // Send the notification
                match client.send(builder.build()?).await {
                    Ok(_) => {
                        // Success - web-push 0.11.0 returns () on success
                        break;
                    }
                    Err(err) => {
                        // Check if the subscription is no longer valid
                        let err_str = err.to_string();

                        // Remove invalid subscriptions (404/410 status codes)
                        if err_str.contains("404")
                            || err_str.contains("410")
                            || err_str.contains("NotFound")
                            || err_str.contains("Gone")
                        {
                            let _ = self.remove_subscription(user_id, &sub.endpoint).await;
                            break;
                        }

                        // Retry on transient errors (5xx, timeouts)
                        if (err_str.contains("500")
                            || err_str.contains("502")
                            || err_str.contains("503")
                            || err_str.contains("timeout")
                            || err_str.contains("ServerError"))
                            && attempts < 3
                        {
                            tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                            backoff_ms *= 2;
                            continue;
                        }

                        // Log error but continue with other subscriptions
                        eprintln!(
                            "Failed to send push notification to {}: {}",
                            sub.endpoint, err
                        );
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}
