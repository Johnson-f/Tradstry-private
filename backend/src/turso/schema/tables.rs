use super::{ManagedTable, TableRenameRule};
use super::ai_table;
use super::brokerage_table;
use super::calendar_table;
use super::notebook_table;
use super::notification_table;
use super::option_table;
use super::playbook_table;
use super::stock_table;

pub const TABLE_RENAME_RULES: &[TableRenameRule] = &[];

pub const MANAGED_TABLES: &[ManagedTable] = &[
    // Stock & trading
    stock_table::STOCKS,
    stock_table::USER_PROFILE,
    option_table::OPTIONS,
    stock_table::STOCK_POSITIONS,
    option_table::OPTION_POSITIONS,
    stock_table::POSITION_TRANSACTIONS,
    stock_table::TRADE_NOTES,
    // Playbook
    playbook_table::PLAYBOOK,
    playbook_table::STOCK_TRADE_PLAYBOOK,
    playbook_table::OPTION_TRADE_PLAYBOOK,
    playbook_table::PLAYBOOK_RULES,
    playbook_table::STOCK_TRADE_RULE_COMPLIANCE,
    playbook_table::OPTION_TRADE_RULE_COMPLIANCE,
    stock_table::MISSED_TRADES,
    // Tags
    stock_table::TRADE_TAGS,
    stock_table::STOCK_TRADE_TAGS,
    option_table::OPTION_TRADE_TAGS,
    // Notebook
    notebook_table::NOTEBOOK_FOLDERS,
    notebook_table::NOTEBOOK_NOTES,
    notebook_table::NOTEBOOK_TAGS,
    notebook_table::NOTEBOOK_NOTE_TAGS,
    notebook_table::NOTEBOOK_IMAGES,
    notebook_table::NOTE_COLLABORATORS,
    notebook_table::NOTE_INVITATIONS,
    notebook_table::NOTE_SHARE_SETTINGS,
    // Notifications & alerts
    notification_table::ALERT_RULES,
    notification_table::PUSH_SUBSCRIPTIONS,
    // AI & chat
    ai_table::CHAT_SESSIONS,
    ai_table::CHAT_MESSAGES,
    ai_table::AI_REPORTS,
    ai_table::AI_ANALYSIS,
    // Brokerage
    brokerage_table::BROKERAGE_CONNECTIONS,
    brokerage_table::BROKERAGE_ACCOUNTS,
    brokerage_table::BROKERAGE_TRANSACTIONS,
    brokerage_table::BROKERAGE_HOLDINGS,
    brokerage_table::UNMATCHED_TRANSACTIONS,
    // Calendar
    calendar_table::CALENDAR_CONNECTIONS,
    calendar_table::CALENDAR_SYNC_CURSORS,
    calendar_table::CALENDAR_EVENTS,
    // Global tables (no user_id)
    notification_table::NOTIFICATIONS,
    notification_table::NOTIFICATION_DISPATCH_LOG,
    notification_table::PUBLIC_NOTES_REGISTRY,
    notification_table::INVITATIONS_REGISTRY,
    notification_table::COLLABORATORS_REGISTRY,
];
