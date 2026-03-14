use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};
use std::sync::Arc;

use crate::service::read_service::{
    accounts as account_service, analytics as analytics_service, journal as journal_service,
    notebook as notebook_service, playbook as playbook_service,
};
use crate::service::turso::TursoClient;

pub async fn execute_tool_call(
    turso: Arc<TursoClient>,
    user_id: &str,
    tool_name: &str,
    arguments: &Value,
) -> Result<Value> {
    let user_db = turso
        .get_user_db(user_id)
        .await
        .with_context(|| format!("failed to open user DB for {}", user_id))?;

    match tool_name {
        "account_summary" => {
            let accounts = account_service::list_accounts(&user_db).await?;
            Ok(json!({ "accounts": accounts }))
        }
        "positions" => Ok(json!({
            "positions": [],
            "status": "unavailable",
            "message": "Brokerage positions are not integrated yet."
        })),
        "recent_trades" => {
            let limit = parse_limit(arguments, 8);
            let mut entries = journal_service::list_journal_entries(&user_db).await?;
            entries.sort_by(|left, right| right.close_date.cmp(&left.close_date));
            entries.truncate(limit);
            Ok(json!({ "trades": entries }))
        }
        "analytics_snapshot" => {
            let accounts = account_service::list_accounts(&user_db).await?;
            let requested_account = arguments
                .get("account_id")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let account_id = requested_account.or_else(|| accounts.first().map(|item| item.id.clone()));
            let Some(account_id) = account_id else {
                return Ok(json!({
                    "analytics": null,
                    "message": "No account available for analytics."
                }));
            };
            let analytics = analytics_service::get_journal_analytics(
                &user_db,
                &account_id,
                &analytics_service::AnalyticsTimeFilter::Last30Days,
            )
            .await?;
            Ok(json!({
                "accountId": account_id,
                "analytics": analytics
            }))
        }
        "journal_entries" => {
            let limit = parse_limit(arguments, 8);
            let mut entries = journal_service::list_journal_entries(&user_db).await?;
            entries.sort_by(|left, right| right.close_date.cmp(&left.close_date));
            entries.truncate(limit);
            Ok(json!({ "entries": entries }))
        }
        "playbook_setups" => {
            let playbooks = playbook_service::list_playbooks(&user_db).await?;
            Ok(json!({ "playbooks": playbooks }))
        }
        "notebook_context" => {
            let limit = parse_limit(arguments, 8);
            let account_id = arguments.get("account_id").and_then(Value::as_str);
            let mut notes = notebook_service::list_notebook_notes(&user_db, account_id).await?;
            notes.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
            notes.truncate(limit);
            Ok(json!({ "notes": notes }))
        }
        other => Err(anyhow!("unsupported tool: {}", other)),
    }
}

fn parse_limit(arguments: &Value, default: usize) -> usize {
    arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(default)
}
