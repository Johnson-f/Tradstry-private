CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY,
    notification_type TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT,
    data TEXT,
    status TEXT NOT NULL DEFAULT 'template',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_notifications_type ON notifications(notification_type);
CREATE INDEX IF NOT EXISTS idx_notifications_status ON notifications(status);
CREATE INDEX IF NOT EXISTS idx_notifications_created_at ON notifications(created_at);

CREATE TRIGGER IF NOT EXISTS update_notifications_timestamp
AFTER UPDATE ON notifications
FOR EACH ROW
BEGIN
    UPDATE notifications SET updated_at = datetime('now') WHERE id = NEW.id;
END;


-- System notifications prompt
-- Mix of motivational and informative notifications to drive user engagement and app adoption
INSERT INTO notifications (id, notification_type, title, body, data, status) VALUES
  (lower(hex(randomblob(16))), 'onboarding', 'Welcome to Tradstry', 'We set up your workspace. Next: connect a broker or import a CSV to start tracking.', '{"action":"open_url","url":"/app/getting-started"}', 'template'),
  (lower(hex(randomblob(16))), 'integration', 'Connect your broker', 'Link your brokerage to sync trades automatically and keep PnL up to date.', '{"action":"open_url","url":"/app/settings/connections"}', 'template'),
  (lower(hex(randomblob(16))), 'data_import', 'Import your first trades', 'Upload a CSV from your broker to backfill your journal.', '{"action":"open_url","url":"/app/import"}', 'template'),
  (lower(hex(randomblob(16))), 'risk', 'Set your risk limits', 'Add max daily loss and position size limits so we can alert you.', '{"action":"open_url","url":"/app/settings/risk"}', 'template'),
  (lower(hex(randomblob(16))), 'alerts', 'Enable price alerts', 'Create alerts for entries, exits, and key levels; we''ll push them to your devices.', '{"action":"open_url","url":"/app/alerts/new"}', 'template'),
  (lower(hex(randomblob(16))), 'journal', 'Log your first trade note', 'Add context, screenshots, and lessons to build a strong review habit.', '{"action":"open_url","url":"/app/journal"}', 'template'),
  (lower(hex(randomblob(16))), 'insight', 'Weekly performance summary is ready', 'Review win rate, PnL, and expectancy for the last 7 days.', '{"action":"open_url","url":"/app/analytics/weekly"}', 'template'),
  (lower(hex(randomblob(16))), 'maintenance', 'Planned maintenance window', 'We will be offline Sat 02:00–02:30 UTC for maintenance.', '{"severity":"info","window_start":"2025-02-01T02:00:00Z","window_end":"2025-02-01T02:30:00Z"}', 'template'),
  (lower(hex(randomblob(16))), 'motivation', 'Consistency beats perfection', 'Traders who review 3+ trades per week see 40% better results. Start your review today.', '{"action":"open_url","url":"/app/journal"}', 'template'),
  (lower(hex(randomblob(16))), 'motivation', 'Your edge is in the data', 'The difference between profitable and losing traders? Detailed trade journals. Log your trades now.', '{"action":"open_url","url":"/app/journal"}', 'template'),
  (lower(hex(randomblob(16))), 'motivation', 'Risk management is wealth management', 'Set your daily loss limit today. Protecting capital is how you compound returns.', '{"action":"open_url","url":"/app/settings/risk"}', 'template'),
  (lower(hex(randomblob(16))), 'motivation', 'You''re 1 analysis away from clarity', 'See your win rate, average win/loss, and PnL drivers. Understanding beats guessing.', '{"action":"open_url","url":"/app/analytics/overview"}', 'template'),
  (lower(hex(randomblob(16))), 'motivation', 'Small habits compound into big wins', 'Just 5 minutes of trade review today builds your edge for tomorrow.', '{"action":"open_url","url":"/app/journal"}', 'template'),
  (lower(hex(randomblob(16))), 'motivation', 'Your trades tell a story', 'Patterns emerge when you track them. Connect your broker and let the data speak.', '{"action":"open_url","url":"/app/settings/connections"}', 'template'),
  (lower(hex(randomblob(16))), 'motivation', 'The best traders are students of their own trades', 'Document your setups, your psychology, your wins and losses. Build mastery.', '{"action":"open_url","url":"/app/journal"}', 'template'),
  (lower(hex(randomblob(16))), 'discipline', 'Discipline > impulses', 'Process beats emotion: follow your plan even when it''s boring.', '{"action":"open_url","url":"/app/journal","slot":"plan"}', 'template'),
  (lower(hex(randomblob(16))), 'discipline', 'Protect capital first', 'Cut losers fast, let winners breathe. Your job is defense before offense.', '{"action":"open_url","url":"/app/settings/risk","slot":"defense"}', 'template'),
  (lower(hex(randomblob(16))), 'discipline', 'Patience pays', 'No setup? No trade. Sitting out is a winning position.', '{"action":"open_url","url":"/app/analytics/overview","slot":"patience"}', 'template'),
  (lower(hex(randomblob(16))), 'discipline', 'Logs make discipline visible', 'Write the trade, tag the mistake, tighten the rule. Journaling keeps you honest.', '{"action":"open_url","url":"/app/journal","slot":"journal"}', 'template');