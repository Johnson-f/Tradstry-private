<!-- 98795a57-2f47-4d78-ab01-a46b4dcbd01b b0218d6f-3af7-4691-829c-b3ce8b36f3e1 -->
# Google Calendar Integration Plan

## Overview

Integrate Google Calendar with OAuth 2.0 authentication, automatic token refresh, cron-based event synchronization, and display external events alongside local events in the calendar UI.

## Backend Implementation

### 1. Environment Configuration

Add OAuth credentials to backend `.env`:

```env
GOOGLE_CLIENT_ID=your_google_client_id
GOOGLE_CLIENT_SECRET=your_google_client_secret
GOOGLE_REDIRECT_URI=http://localhost:3000/api/auth/callback/google
```

### 2. Enhance Calendar Service (`backend/src/service/calendar_service.rs`)

- Add token refresh method for Google
- Implement automatic token refresh before API calls
- Add error handling for expired tokens with auto-refresh retry
- Update `sync_google_events` to check token expiry before syncing

Key additions:

```rust
pub async fn refresh_google_token(refresh_token: &str, client_id: &str, client_secret: &str) -> Result<(String, String, String)>
pub async fn ensure_valid_token(conn: &Connection, connection_id: &str, client_id: &str, client_secret: &str) -> Result<String>
```

Token refresh implementation:

```rust
pub async fn refresh_google_token(refresh_token: &str, client_id: &str, client_secret: &str) -> Result<(String, String, String)> {
    let client = Client::new();
    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("refresh_token", refresh_token),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?;
    let json: serde_json::Value = resp.json().await?;
    let access = json.get("access_token").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let expires_in = json.get("expires_in").and_then(|v| v.as_i64()).unwrap_or(3600);
    let expiry = (Utc::now() + chrono::Duration::seconds(expires_in)).to_rfc3339();
    Ok((access, refresh_token.to_string(), expiry))
}
```

### 3. Add Cron Sync Endpoint (`backend/src/routes/notebook.rs`)

Create new endpoint for external cron job:

```rust
POST /api/notebook/calendar/sync-all
```

- Authenticate with service token (add CRON_SECRET to .env)
- Fetch all active Google calendar connections from registry database
- For each user, get their database connection and sync their calendars
- Return sync statistics (success/failure counts, total events synced)
- Add rate limiting and error handling

Implementation:

```rust
async fn sync_all_calendars(
    req: HttpRequest,
    turso_client: web::Data<Arc<TursoClient>>,
) -> Result<HttpResponse> {
    // Verify cron secret from header
    let cron_secret = req.headers().get("X-Cron-Secret")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Missing cron secret"))?;
    
    if cron_secret != std::env::var("CRON_SECRET").unwrap_or_default() {
        return Err(actix_web::error::ErrorUnauthorized("Invalid cron secret"));
    }
    
    // Get all users with active connections from registry
    // For each user, sync their calendars
    // Return statistics
}
```

### 4. Update Route Configuration

Register new routes in `backend/src/routes/notebook.rs`:

```rust
.route("/calendar/connections", web::get().to(list_calendar_connections))
.route("/calendar/connections/{id}", web::delete().to(disconnect_calendar))
.route("/calendar/connections/{id}/sync", web::post().to(sync_calendar))
.route("/calendar/events", web::get().to(get_calendar_events))
.route("/calendar/sync-all", web::post().to(sync_all_calendars))
.route("/oauth/google/exchange", web::post().to(google_oauth_exchange))
```

Add new handler for listing connections:

```rust
async fn list_calendar_connections(
    req: HttpRequest,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let conn = get_user_database_connection(&claims.sub, &turso_client).await?;
    
    let mut stmt = conn.prepare("SELECT * FROM external_calendar_connections WHERE provider = 'google' AND is_active = 1").await?;
    let mut rows = stmt.query(params![]).await?;
    let mut connections = Vec::new();
    
    while let Some(row) = rows.next().await? {
        connections.push(/* parse connection */);
    }
    
    Ok(HttpResponse::Ok().json(connections))
}
```

Add handler for getting calendar events:

```rust
async fn get_calendar_events(
    req: HttpRequest,
    query: web::Query<DateRangeQuery>,
    turso_client: web::Data<Arc<TursoClient>>,
    supabase_config: web::Data<SupabaseConfig>,
) -> Result<HttpResponse> {
    let claims = get_authenticated_user(&req, &supabase_config).await?;
    let conn = get_user_database_connection(&claims.sub, &turso_client).await?;
    
    let events = ExternalCalendarEvent::find_by_date_range(&conn, &query.start, &query.end).await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to fetch events"))?;
    
    Ok(HttpResponse::Ok().json(events))
}
```

### 5. Add Model Methods (`backend/src/models/notebook/calendar.rs`)

Extend models with:

```rust
impl ExternalCalendarConnection {
    pub async fn find_by_user(conn: &Connection) -> Result<Vec<Self>> {
        let mut stmt = conn.prepare(
            "SELECT * FROM external_calendar_connections WHERE provider = 'google' AND is_active = 1"
        ).await?;
        let mut rows = stmt.query(params![]).await?;
        let mut connections = Vec::new();
        while let Some(row) = rows.next().await? {
            connections.push(Self::from_row(row)?);
        }
        Ok(connections)
    }
    
    pub async fn update_tokens(conn: &Connection, id: &str, access_token: &str, refresh_token: &str, expiry: &str) -> Result<()> {
        conn.execute(
            "UPDATE external_calendar_connections SET access_token = ?, refresh_token = ?, token_expiry = ?, updated_at = datetime('now') WHERE id = ?",
            params![access_token, refresh_token, expiry, id],
        ).await?;
        Ok(())
    }
    
    fn from_row(row: libsql::Row) -> Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            provider: row.get(1)?,
            access_token: row.get(2)?,
            refresh_token: row.get(3)?,
            token_expiry: row.get(4)?,
            calendar_id: row.get(5)?,
            is_active: row.get::<i64>(6)? != 0,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    }
}

impl ExternalCalendarEvent {
    pub async fn find_by_connection(conn: &Connection, connection_id: &str) -> Result<Vec<Self>> {
        let mut stmt = conn.prepare(
            "SELECT * FROM external_calendar_events WHERE connection_id = ? ORDER BY start_time ASC"
        ).await?;
        let mut rows = stmt.query(params![connection_id]).await?;
        let mut events = Vec::new();
        while let Some(row) = rows.next().await? {
            events.push(Self::from_row(row)?);
        }
        Ok(events)
    }
    
    pub async fn find_by_date_range(conn: &Connection, start: &str, end: &str) -> Result<Vec<Self>> {
        let mut stmt = conn.prepare(
            "SELECT * FROM external_calendar_events WHERE start_time >= ? AND start_time <= ? ORDER BY start_time ASC"
        ).await?;
        let mut rows = stmt.query(params![start, end]).await?;
        let mut events = Vec::new();
        while let Some(row) = rows.next().await? {
            events.push(Self::from_row(row)?);
        }
        Ok(events)
    }
    
    fn from_row(row: libsql::Row) -> Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            connection_id: row.get(1)?,
            external_event_id: row.get(2)?,
            title: row.get(3)?,
            description: row.get(4)?,
            start_time: row.get(5)?,
            end_time: row.get(6)?,
            location: row.get(7)?,
            last_synced_at: row.get(8)?,
        })
    }
}
```

## Frontend Implementation

### 6. Create Calendar Service (`lib/services/calendar-service.ts`)

```typescript
import { API_CONFIG } from '@/lib/config/api';

export interface CalendarConnection {
  id: string;
  provider: 'google';
  calendar_id?: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface ExternalCalendarEvent {
  id: string;
  connection_id: string;
  external_event_id: string;
  title: string;
  description?: string;
  start_time: string;
  end_time: string;
  location?: string;
}

export interface SyncResult {
  success: boolean;
  synced: number;
}

export class CalendarService {
  static async initiateGoogleOAuth(): Promise<void> {
    const clientId = process.env.NEXT_PUBLIC_GOOGLE_CLIENT_ID;
    const redirectUri = `${window.location.origin}/api/auth/callback/google`;
    const scope = 'https://www.googleapis.com/auth/calendar.readonly';
    
    const authUrl = `https://accounts.google.com/o/oauth2/v2/auth?` +
      `client_id=${clientId}&` +
      `redirect_uri=${encodeURIComponent(redirectUri)}&` +
      `response_type=code&` +
      `scope=${encodeURIComponent(scope)}&` +
      `access_type=offline&` +
      `prompt=consent`;
    
    window.location.href = authUrl;
  }
  
  static async getConnections(): Promise<CalendarConnection[]> {
    const response = await fetch(`${API_CONFIG.BASE_URL}/notebook/calendar/connections`, {
      credentials: 'include',
    });
    if (!response.ok) throw new Error('Failed to fetch connections');
    return response.json();
  }
  
  static async disconnectCalendar(connectionId: string): Promise<void> {
    const response = await fetch(`${API_CONFIG.BASE_URL}/notebook/calendar/connections/${connectionId}`, {
      method: 'DELETE',
      credentials: 'include',
    });
    if (!response.ok) throw new Error('Failed to disconnect');
  }
  
  static async syncCalendar(connectionId: string): Promise<SyncResult> {
    const response = await fetch(`${API_CONFIG.BASE_URL}/notebook/calendar/connections/${connectionId}/sync`, {
      method: 'POST',
      credentials: 'include',
    });
    if (!response.ok) throw new Error('Failed to sync');
    return response.json();
  }
  
  static async getExternalEvents(startDate: Date, endDate: Date): Promise<ExternalCalendarEvent[]> {
    const start = startDate.toISOString();
    const end = endDate.toISOString();
    const response = await fetch(
      `${API_CONFIG.BASE_URL}/notebook/calendar/events?start=${start}&end=${end}`,
      { credentials: 'include' }
    );
    if (!response.ok) throw new Error('Failed to fetch events');
    return response.json();
  }
}
```

### 7. Create OAuth Callback Route

**`app/api/auth/callback/google/route.ts`**

```typescript
import { NextRequest, NextResponse } from 'next/server';
import { API_CONFIG } from '@/lib/config/api';

export async function GET(request: NextRequest) {
  const searchParams = request.nextUrl.searchParams;
  const code = searchParams.get('code');
  const error = searchParams.get('error');
  
  if (error) {
    return NextResponse.redirect(new URL('/app/notebook/calendar?error=oauth_failed', request.url));
  }
  
  if (!code) {
    return NextResponse.redirect(new URL('/app/notebook/calendar?error=no_code', request.url));
  }
  
  try {
    // Exchange code for tokens via backend
    const response = await fetch(`${API_CONFIG.BASE_URL}/notebook/oauth/google/exchange`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Cookie': request.headers.get('cookie') || '',
      },
      body: JSON.stringify({
        code,
        client_id: process.env.GOOGLE_CLIENT_ID,
        client_secret: process.env.GOOGLE_CLIENT_SECRET,
        redirect_uri: `${request.nextUrl.origin}/api/auth/callback/google`,
      }),
    });
    
    if (!response.ok) {
      throw new Error('Token exchange failed');
    }
    
    return NextResponse.redirect(new URL('/app/notebook/calendar?success=google_connected', request.url));
  } catch (error) {
    console.error('OAuth callback error:', error);
    return NextResponse.redirect(new URL('/app/notebook/calendar?error=exchange_failed', request.url));
  }
}
```

### 8. Create Calendar Connections Hook (`hooks/use-calendar-connections.ts`)

```typescript
import { useState, useEffect, useCallback } from 'react';
import { CalendarService, CalendarConnection, SyncResult } from '@/lib/services/calendar-service';
import { toast } from 'sonner';

export function useCalendarConnections() {
  const [connections, setConnections] = useState<CalendarConnection[]>([]);
  const [loading, setLoading] = useState(false);
  
  const fetchConnections = useCallback(async () => {
    try {
      setLoading(true);
      const data = await CalendarService.getConnections();
      setConnections(data);
    } catch (error) {
      console.error('Failed to fetch connections:', error);
      toast.error('Failed to load calendar connections');
    } finally {
      setLoading(false);
    }
  }, []);
  
  useEffect(() => {
    fetchConnections();
  }, [fetchConnections]);
  
  const connectGoogle = useCallback(async () => {
    try {
      await CalendarService.initiateGoogleOAuth();
    } catch (error) {
      console.error('Failed to initiate OAuth:', error);
      toast.error('Failed to connect Google Calendar');
    }
  }, []);
  
  const disconnect = useCallback(async (id: string) => {
    try {
      await CalendarService.disconnectCalendar(id);
      toast.success('Calendar disconnected');
      fetchConnections();
    } catch (error) {
      console.error('Failed to disconnect:', error);
      toast.error('Failed to disconnect calendar');
    }
  }, [fetchConnections]);
  
  const syncConnection = useCallback(async (id: string) => {
    try {
      const result = await CalendarService.syncCalendar(id);
      toast.success(`Synced ${result.synced} events`);
      return result;
    } catch (error) {
      console.error('Failed to sync:', error);
      toast.error('Failed to sync calendar');
      throw error;
    }
  }, []);
  
  return { 
    connections, 
    loading, 
    connectGoogle, 
    disconnect, 
    syncConnection,
    refetch: fetchConnections 
  };
}
```

### 9. Create External Events Hook (`hooks/use-external-events.ts`)

```typescript

import { useState, useEffect, useCallback } from 'react';

import { CalendarService, ExternalCalendarEvent } from '@/lib/services/calendar-service';

import { startOfDay, endOfDay } from 'date-fns';

export function useExternalEvents(selectedDate: Date) {

const [events, setEvents] = useState<ExternalCalendarEvent[]>

### To-dos

- [x] Add OAuth credentials to backend .env file
- [ ] Add token refresh methods to CalendarService
- [ ] Create sync-all endpoint for external cron job
- [ ] Register all calendar-related routes in notebook.rs
- [ ] Add model methods for connections and events
- [ ] Create CalendarService class in frontend
- [ ] Create OAuth callback routes for Google and Microsoft
- [ ] Create useCalendarConnections hook
- [ ] Create useExternalEvents hook
- [ ] Update Calendar component to fetch and display external events
- [ ] Update DayView to render external events with provider badges
- [ ] Make Connect calendar buttons functional with OAuth
- [ ] Create CalendarSettingsModal component
- [ ] Create calendar type definitions
- [ ] Configure external cron job for periodic sync