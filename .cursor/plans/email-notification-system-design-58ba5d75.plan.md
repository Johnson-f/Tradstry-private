<!-- 58ba5d75-b1ae-4f22-b4fb-8abfa3b69ad1 fd49c6e6-e843-4321-9b05-7ce86f5aa91e -->
# Email Notification System Design

## Overview

Design a comprehensive external notification system using useSend for emails and Web Push API for browser push notifications. The system integrates with the existing Tradistry trading platform to support transactional emails, push notifications, user preferences, notification queuing, and delivery tracking for both channels.

## Architecture Components

### 1. Database Schema

#### Central Registry Database (`user_databases` table)

Add notification preferences table:

```sql
-- User notification preferences (stored in registry DB)
CREATE TABLE IF NOT EXISTS notification_preferences (
  user_id TEXT PRIMARY KEY,
  email_enabled BOOLEAN NOT NULL DEFAULT true,
  email_verified BOOLEAN NOT NULL DEFAULT false,
  email_address TEXT NOT NULL,
  push_enabled BOOLEAN NOT NULL DEFAULT false,
  notification_types TEXT NOT NULL DEFAULT '[]', -- JSON array of enabled notification types
  frequency TEXT NOT NULL DEFAULT 'immediate', -- 'immediate', 'daily', 'weekly'
  quiet_hours_start TIME, -- e.g., '22:00'
  quiet_hours_end TIME,   -- e.g., '08:00'
  timezone TEXT DEFAULT 'UTC',
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_notification_preferences_email ON notification_preferences(email_address);
```

#### Push Subscription Table (Registry DB)

```sql
-- Browser push notification subscriptions (Web Push API)
CREATE TABLE IF NOT EXISTS push_subscriptions (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  endpoint TEXT NOT NULL UNIQUE, -- Web Push endpoint URL
  p256dh TEXT NOT NULL, -- Public key for encryption
  auth TEXT NOT NULL, -- Auth secret for encryption
  user_agent TEXT,
  device_info TEXT, -- JSON: platform, browser, etc.
  is_active BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  last_used_at TIMESTAMP,
  FOREIGN KEY (user_id) REFERENCES notification_preferences(user_id)
);

CREATE INDEX IF NOT EXISTS idx_push_subscriptions_user ON push_subscriptions(user_id, is_active);
CREATE INDEX IF NOT EXISTS idx_push_subscriptions_endpoint ON push_subscriptions(endpoint);
```

#### Notification Queue Table (Registry DB)

```sql
-- Unified notification queue (supports both email and push)
CREATE TABLE IF NOT EXISTS notification_queue (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  notification_type TEXT NOT NULL, -- 'trade_created', 'trade_closed', 'reminder', etc.
  channels TEXT NOT NULL DEFAULT '[]', -- JSON array: ['email', 'push'] or ['email'] or ['push']
  subject TEXT NOT NULL,
  body_html TEXT, -- For email
  body_text TEXT, -- For email
  title TEXT NOT NULL, -- For push notifications
  message TEXT NOT NULL, -- For push notifications
  icon_url TEXT, -- For push notifications
  badge_url TEXT, -- For push notifications
  data TEXT, -- JSON payload for push notification click action
  priority TEXT NOT NULL DEFAULT 'normal', -- 'low', 'normal', 'high'
  status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'processing', 'sent', 'failed', 'cancelled'
  attempts INTEGER NOT NULL DEFAULT 0,
  max_attempts INTEGER NOT NULL DEFAULT 3,
  scheduled_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  sent_at TIMESTAMP,
  error_message TEXT,
  metadata TEXT, -- JSON for additional data
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (user_id) REFERENCES notification_preferences(user_id)
);

CREATE INDEX IF NOT EXISTS idx_notification_queue_status ON notification_queue(status, scheduled_at);
CREATE INDEX IF NOT EXISTS idx_notification_queue_user ON notification_queue(user_id, status);
```

#### Notification History Table (Registry DB)

```sql
-- Unified notification delivery history (email and push)
CREATE TABLE IF NOT EXISTS notification_history (
  id TEXT PRIMARY KEY,
  queue_id TEXT NOT NULL,
  user_id TEXT NOT NULL,
  notification_type TEXT NOT NULL,
  channel TEXT NOT NULL, -- 'email' or 'push'
  subject TEXT, -- For email
  title TEXT, -- For push
  status TEXT NOT NULL, -- 'sent', 'delivered', 'opened', 'clicked', 'bounced', 'failed'
  external_id TEXT, -- useSend email ID or push notification ID
  delivered_at TIMESTAMP,
  opened_at TIMESTAMP,
  clicked_at TIMESTAMP,
  bounced_at TIMESTAMP,
  error_message TEXT,
  metadata TEXT, -- JSON
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (queue_id) REFERENCES notification_queue(id),
  FOREIGN KEY (user_id) REFERENCES notification_preferences(user_id)
);

CREATE INDEX IF NOT EXISTS idx_notification_history_user ON notification_history(user_id, created_at);
CREATE INDEX IF NOT EXISTS idx_notification_history_status ON notification_history(status);
CREATE INDEX IF NOT EXISTS idx_notification_history_channel ON notification_history(channel);
```

### 2. Backend Service Layer

#### Notification Service (`backend/src/service/notification_service.rs`)

```rust
pub struct NotificationService {
    app_state: web::Data<AppState>,
    use_send_client: UseSendClient,
}

impl NotificationService {
    // Queue notification for sending
    pub async fn queue_notification(
        &self,
        user_id: &str,
        notification_type: NotificationType,
        subject: String,
        body_html: String,
        body_text: Option<String>,
        priority: Priority,
        scheduled_at: Option<DateTime<Utc>>,
        metadata: Option<serde_json::Value>,
    ) -> Result<String>; // Returns queue ID

    // Process pending notifications
    pub async fn process_queue(&self) -> Result<()>;

    // Send email via useSend API
    pub async fn send_email(&self, queue_item: &NotificationQueueItem) -> Result<String>;

    // Update notification preferences
    pub async fn update_preferences(
        &self,
        user_id: &str,
        preferences: NotificationPreferences,
    ) -> Result<()>;

    // Handle useSend webhook events
    pub async fn handle_webhook(&self, event: UseSendWebhookEvent) -> Result<()>;
}
```

#### UseSend Client (`backend/src/service/use_send_client.rs`)

```rust
pub struct UseSendClient {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl UseSendClient {
    // Send email via useSend API
    pub async fn send_email(&self, request: SendEmailRequest) -> Result<SendEmailResponse>;

    // Get email status
    pub async fn get_email_status(&self, email_id: &str) -> Result<EmailStatus>;
}
```

### 3. Configuration

#### Environment Variables (`backend/env.template`)

```bash
# useSend Configuration
USESEND_API_KEY=your-usesend-api-key
USESEND_API_URL=https://app.usesend.com/api
USESEND_WEBHOOK_SECRET=your-usesend-webhook-secret
USESEND_FROM_EMAIL=noreply@tradstry.com
USESEND_FROM_NAME=Tradstry

# Notification Settings
NOTIFICATION_BATCH_SIZE=50
NOTIFICATION_RETRY_DELAY_SECONDS=300
NOTIFICATION_MAX_ATTEMPTS=3
```

#### Config Struct (`backend/src/turso/config.rs`)

Add useSend configuration:

```rust
pub struct UseSendConfig {
    pub api_key: String,
    pub api_url: String,
    pub webhook_secret: String,
    pub from_email: String,
    pub from_name: String,
}

pub struct NotificationConfig {
    pub batch_size: usize,
    pub retry_delay_seconds: u64,
    pub max_attempts: u32,
}
```

### 4. API Routes

#### Notification Routes (`backend/src/routes/notifications.rs`)

```rust
pub fn configure_notification_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/notifications")
            // User preferences
            .route("/preferences", web::get().to(get_notification_preferences))
            .route("/preferences", web::put().to(update_notification_preferences))
            // Notification history
            .route("/history", web::get().to(get_notification_history))
            // Webhook handler for useSend events
            .route("/webhook/usesend", web::post().to(usesend_webhook_handler))
    );
}
```

### 5. Integration Points

#### Trigger Points (where notifications are queued)

- **Trade Events**: When trades are created/updated/closed (`backend/src/routes/stocks.rs`, `backend/src/routes/options.rs`)
- **Reminder Events**: When reminders are created/updated (`backend/src/routes/notebook.rs`)
- **Playbook Events**: When playbooks match trades (`backend/src/routes/playbook.rs`)
- **Account Events**: When user profile changes (`backend/src/routes/user.rs`)

#### WebSocket Integration

Extend existing WebSocket system to queue email notifications alongside in-app notifications:

- Modify `backend/src/websocket/broadcast.rs` to queue emails when broadcasting events
- Add notification type checking before queueing

### 6. Background Processing

#### Queue Processor (`backend/src/service/notification_queue_processor.rs`)

```rust
pub struct NotificationQueueProcessor {
    notification_service: Arc<NotificationService>,
}

impl NotificationQueueProcessor {
    // Background task to process pending notifications
    pub async fn start_processor(&self) -> Result<()>;
    
    // Process batch of notifications
    pub async fn process_batch(&self) -> Result<usize>;
}
```

#### Cron Job Integration

Add periodic task to process notification queue:

- Run every 1-5 minutes
- Process pending notifications in batches
- Handle retries for failed notifications

### 7. Frontend Integration

#### Notification Preferences UI (`app/app/settings/notifications/page.tsx`)

- User preferences form
- Enable/disable notification types
- Set frequency preferences
- Configure quiet hours
- View notification history

#### Hooks (`hooks/use-notifications.ts`)

```typescript
export function useNotificationPreferences() {
  // Fetch and update notification preferences
}

export function useNotificationHistory() {
  // Fetch notification history
}
```

### 8. Email Templates

#### Template System (`backend/src/service/email_templates.rs`)

```rust
pub struct EmailTemplate {
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
}

impl EmailTemplate {
    // Render template with data
    pub fn render(&self, data: &HashMap<String, String>) -> Result<Self>;
    
    // Predefined templates
    pub fn trade_created(trade: &Trade) -> Self;
    pub fn trade_closed(trade: &Trade) -> Self;
    pub fn reminder(reminder: &Reminder) -> Self;
    // ... more templates
}
```

### 9. Error Handling & Retry Logic

- Exponential backoff for failed sends
- Dead letter queue for permanently failed notifications
- Error logging and alerting
- Rate limiting to respect useSend limits

### 10. Monitoring & Analytics

- Track delivery rates
- Monitor bounce rates
- Track open/click rates via useSend webhooks
- Alert on high failure rates

## Implementation Order

1. **Phase 1: Foundation**

   - Database schema migration
   - useSend client implementation
   - Configuration setup

2. **Phase 2: Core Service**

   - Notification service implementation
   - Queue management
   - Basic email sending

3. **Phase 3: Integration**

   - API routes
   - Integration with existing trigger points
   - Webhook handler

4. **Phase 4: Processing**

   - Background queue processor
   - Retry logic
   - Error handling

5. **Phase 5: Frontend**

   - Preferences UI
   - History view
   - User hooks

6. **Phase 6: Templates & Polish**

   - Email templates
   - Monitoring
   - Analytics

## Files to Create/Modify

### New Files

- `backend/src/service/notification_service.rs`
- `backend/src/service/use_send_client.rs`
- `backend/src/service/notification_queue_processor.rs`
- `backend/src/service/email_templates.rs`
- `backend/src/models/notification.rs`
- `backend/src/routes/notifications.rs`
- `backend/database/08_notifications/01_notification_tables.sql`
- `app/app/settings/notifications/page.tsx`
- `hooks/use-notifications.ts`

### Modified Files

- `backend/src/turso/config.rs` - Add useSend config
- `backend/src/main.rs` - Register notification routes
- `backend/src/routes/mod.rs` - Export notification routes
- `backend/src/service/mod.rs` - Export notification service
- `backend/env.template` - Add useSend env vars
- `backend/src/routes/stocks.rs` - Queue notifications on trade events
- `backend/src/routes/options.rs` - Queue notifications on option events
- `backend/src/routes/notebook.rs` - Queue notifications on reminder events

## Dependencies

### Backend (Cargo.toml)

```toml
reqwest = { version = "0.11", features = ["json"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
```

## Security Considerations

- Validate webhook signatures from useSend
- Sanitize email content to prevent injection
- Rate limit notification endpoints
- Store API keys securely
- Validate user permissions before sending notifications
- Implement email verification before sending

## Future Enhancements

- SMS notifications (via useSend when available)
- Push notifications
- Digest emails (daily/weekly summaries)
- Unsubscribe handling
- A/B testing for email templates
- Preference synchronization with in-app notifications