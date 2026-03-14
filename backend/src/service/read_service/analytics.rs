use crate::service::turso::client::UserDb;
use crate::service::turso::schema::tables::journal_table::{self, JournalEntry};

use anyhow::{Result, anyhow, ensure};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, Utc};
use std::cmp::Ordering;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct TradeOutcome {
    pub symbol: String,
    pub symbol_name: String,
    pub amount: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JournalAnalytics {
    pub win_rate: f64,
    pub cumulative_profit: f64,
    pub average_risk_to_reward: f64,
    pub average_gain: f64,
    pub average_loss: f64,
    pub profit_factor: Option<f64>,
    pub biggest_win: Option<TradeOutcome>,
    pub biggest_loss: Option<TradeOutcome>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalendarDaySummary {
    pub date: String,
    pub profit: f64,
    pub trade_count: usize,
    pub win_rate: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalendarWeekSummary {
    pub week_index: usize,
    pub week_start: String,
    pub week_end: String,
    pub profit: f64,
    pub trade_count: usize,
    pub trading_days: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalendarAnalytics {
    pub year: i32,
    pub month: u32,
    pub month_profit: f64,
    pub trade_count: usize,
    pub trading_days: usize,
    pub grid_start: String,
    pub grid_end: String,
    pub days: Vec<CalendarDaySummary>,
    pub weeks: Vec<CalendarWeekSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalyticsTimeFilter {
    Last7Days,
    Last30Days,
    YearToDate,
    Last1Year,
    Custom {
        start_date: String,
        end_date: String,
    },
}

pub async fn get_journal_analytics(
    user_db: &UserDb,
    account_id: &str,
    time_filter: &AnalyticsTimeFilter,
) -> Result<JournalAnalytics> {
    let entries = journal_table::list_journal_entries(user_db.conn(), user_db.user_id()).await?;
    let account_entries = entries
        .into_iter()
        .filter(|entry| entry.account_id == account_id)
        .collect::<Vec<_>>();
    let time_filtered_entries = filter_entries_by_time(account_entries, time_filter, Utc::now())?;

    Ok(calculate_journal_analytics(time_filtered_entries))
}

pub async fn get_calendar_analytics(
    user_db: &UserDb,
    account_id: &str,
    year: i32,
    month: u32,
) -> Result<CalendarAnalytics> {
    let entries = journal_table::list_journal_entries(user_db.conn(), user_db.user_id()).await?;
    let account_entries = entries
        .into_iter()
        .filter(|entry| entry.account_id == account_id)
        .collect::<Vec<_>>();

    calculate_calendar_analytics(account_entries, year, month)
}

fn calculate_journal_analytics(
    entries: impl IntoIterator<Item = JournalEntry>,
) -> JournalAnalytics {
    let entries = entries.into_iter().collect::<Vec<_>>();
    if entries.is_empty() {
        return JournalAnalytics {
            win_rate: 0.0,
            cumulative_profit: 0.0,
            average_risk_to_reward: 0.0,
            average_gain: 0.0,
            average_loss: 0.0,
            profit_factor: None,
            biggest_win: None,
            biggest_loss: None,
        };
    }

    let total_trades = entries.len() as f64;
    let cumulative_profit = entries.iter().map(dollar_pl).sum::<f64>();
    let average_risk_to_reward =
        entries.iter().map(|entry| entry.risk_reward).sum::<f64>() / total_trades;

    let winning_trades = entries
        .iter()
        .map(|entry| (entry, dollar_pl(entry)))
        .filter(|(_, amount)| *amount > 0.0)
        .collect::<Vec<_>>();
    let losing_trades = entries
        .iter()
        .map(|entry| (entry, dollar_pl(entry)))
        .filter(|(_, amount)| *amount < 0.0)
        .collect::<Vec<_>>();

    let win_rate = (winning_trades.len() as f64 / total_trades) * 100.0;
    let average_gain = if winning_trades.is_empty() {
        0.0
    } else {
        winning_trades
            .iter()
            .map(|(_, amount)| *amount)
            .sum::<f64>()
            / winning_trades.len() as f64
    };
    let average_loss = if losing_trades.is_empty() {
        0.0
    } else {
        losing_trades
            .iter()
            .map(|(_, amount)| amount.abs())
            .sum::<f64>()
            / losing_trades.len() as f64
    };

    let gross_profit = winning_trades
        .iter()
        .map(|(_, amount)| *amount)
        .sum::<f64>();
    let gross_loss = losing_trades
        .iter()
        .map(|(_, amount)| amount.abs())
        .sum::<f64>();
    let profit_factor = if gross_loss > 0.0 {
        Some(gross_profit / gross_loss)
    } else {
        None
    };

    let biggest_win = winning_trades
        .iter()
        .max_by(|(_, left_amount), (_, right_amount)| compare_f64(*left_amount, *right_amount))
        .map(|(entry, amount)| TradeOutcome {
            symbol: entry.symbol.clone(),
            symbol_name: entry.symbol_name.clone(),
            amount: *amount,
        });
    let biggest_loss = losing_trades
        .iter()
        .min_by(|(_, left_amount), (_, right_amount)| compare_f64(*left_amount, *right_amount))
        .map(|(entry, amount)| TradeOutcome {
            symbol: entry.symbol.clone(),
            symbol_name: entry.symbol_name.clone(),
            amount: *amount,
        });

    JournalAnalytics {
        win_rate,
        cumulative_profit,
        average_risk_to_reward,
        average_gain,
        average_loss,
        profit_factor,
        biggest_win,
        biggest_loss,
    }
}

fn calculate_calendar_analytics(
    entries: impl IntoIterator<Item = JournalEntry>,
    year: i32,
    month: u32,
) -> Result<CalendarAnalytics> {
    ensure!((1..=12).contains(&month), "month must be between 1 and 12");

    let month_start =
        NaiveDate::from_ymd_opt(year, month, 1).ok_or_else(|| anyhow!("Invalid calendar month"))?;
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).ok_or_else(|| anyhow!("Invalid next month"))?
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).ok_or_else(|| anyhow!("Invalid next month"))?
    };
    let month_end = next_month - Duration::days(1);
    let grid_start = start_of_calendar_week(month_start);
    let grid_end = end_of_calendar_week(month_end);

    let mut entries_by_day: BTreeMap<NaiveDate, Vec<JournalEntry>> = BTreeMap::new();
    for entry in entries {
        let close_date = parse_trade_datetime(&entry.close_date)?.date_naive();
        if close_date >= month_start && close_date <= month_end {
            entries_by_day.entry(close_date).or_default().push(entry);
        }
    }

    let mut days = Vec::new();
    let mut cursor = month_start;
    while cursor <= month_end {
        let daily_entries = entries_by_day.get(&cursor).cloned().unwrap_or_default();
        days.push(calculate_calendar_day_summary(cursor, &daily_entries));
        cursor += Duration::days(1);
    }

    let month_profit = days.iter().map(|day| day.profit).sum::<f64>();
    let trade_count = days.iter().map(|day| day.trade_count).sum::<usize>();
    let trading_days = days.iter().filter(|day| day.trade_count > 0).count();

    let mut weeks = Vec::new();
    let mut week_start = grid_start;
    let mut week_index = 1;
    while week_start <= grid_end {
        let week_end = week_start + Duration::days(6);
        let week_days = days
            .iter()
            .filter(|day| {
                NaiveDate::parse_from_str(&day.date, "%Y-%m-%d")
                    .map(|date| date >= week_start && date <= week_end)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();

        weeks.push(CalendarWeekSummary {
            week_index,
            week_start: week_start.format("%Y-%m-%d").to_string(),
            week_end: week_end.format("%Y-%m-%d").to_string(),
            profit: week_days.iter().map(|day| day.profit).sum::<f64>(),
            trade_count: week_days.iter().map(|day| day.trade_count).sum::<usize>(),
            trading_days: week_days.iter().filter(|day| day.trade_count > 0).count(),
        });

        week_index += 1;
        week_start += Duration::days(7);
    }

    Ok(CalendarAnalytics {
        year,
        month,
        month_profit,
        trade_count,
        trading_days,
        grid_start: grid_start.format("%Y-%m-%d").to_string(),
        grid_end: grid_end.format("%Y-%m-%d").to_string(),
        days,
        weeks,
    })
}

fn dollar_pl(entry: &JournalEntry) -> f64 {
    entry.position_size * entry.total_pl / 100.0
}

fn calculate_calendar_day_summary(date: NaiveDate, entries: &[JournalEntry]) -> CalendarDaySummary {
    let profit = entries.iter().map(dollar_pl).sum::<f64>();
    let trade_count = entries.len();
    let winning_trades = entries
        .iter()
        .filter(|entry| dollar_pl(entry) > 0.0)
        .count();

    CalendarDaySummary {
        date: date.format("%Y-%m-%d").to_string(),
        profit,
        trade_count,
        win_rate: if trade_count == 0 {
            0.0
        } else {
            (winning_trades as f64 / trade_count as f64) * 100.0
        },
    }
}

fn filter_entries_by_time(
    entries: Vec<JournalEntry>,
    time_filter: &AnalyticsTimeFilter,
    now: DateTime<Utc>,
) -> Result<Vec<JournalEntry>> {
    let (start, end) = resolve_time_bounds(time_filter, now)?;

    Ok(entries
        .into_iter()
        .filter(|entry| match parse_trade_datetime(&entry.close_date) {
            Ok(close_date) => close_date >= start && close_date <= end,
            Err(e) => {
                eprintln!(
                    "[analytics] failed to parse close_date '{}' for entry '{}': {e}",
                    entry.close_date, entry.symbol
                );
                false
            }
        })
        .collect())
}

fn resolve_time_bounds(
    time_filter: &AnalyticsTimeFilter,
    now: DateTime<Utc>,
) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    // Allow trades closed any time up to 1 year in the future so
    // forward-dated journal entries (e.g. close_date logged ahead of time)
    // are always included in range filters.
    let far_future = now + Duration::days(366);

    match time_filter {
        AnalyticsTimeFilter::Last7Days => Ok((now - Duration::days(7), far_future)),
        AnalyticsTimeFilter::Last30Days => Ok((now - Duration::days(30), far_future)),
        AnalyticsTimeFilter::YearToDate => {
            let start = NaiveDate::from_ymd_opt(now.year(), 1, 1)
                .and_then(|date| date.and_hms_opt(0, 0, 0))
                .ok_or_else(|| anyhow!("Failed to compute year-to-date start"))?;
            Ok((
                DateTime::<Utc>::from_naive_utc_and_offset(start, Utc),
                far_future,
            ))
        }
        AnalyticsTimeFilter::Last1Year => Ok((now - Duration::days(365), far_future)),
        AnalyticsTimeFilter::Custom {
            start_date,
            end_date,
        } => {
            let start = parse_filter_datetime(start_date, FilterBound::Start)?;
            let end = parse_filter_datetime(end_date, FilterBound::End)?;
            ensure!(
                end >= start,
                "custom end_date must be on or after start_date"
            );
            Ok((start, end))
        }
    }
}

fn compare_f64(left: f64, right: f64) -> Ordering {
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

fn start_of_calendar_week(date: NaiveDate) -> NaiveDate {
    let days_from_sunday = i64::from(date.weekday().num_days_from_sunday());
    date - Duration::days(days_from_sunday)
}

fn end_of_calendar_week(date: NaiveDate) -> NaiveDate {
    let days_until_saturday = 6_i64 - i64::from(date.weekday().num_days_from_sunday());
    date + Duration::days(days_until_saturday)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilterBound {
    Start,
    End,
}

fn parse_trade_datetime(value: &str) -> Result<DateTime<Utc>> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Ok(parsed.with_timezone(&Utc));
    }

    for format in [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%dT%H:%M",
    ] {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(value, format) {
            return Ok(DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc));
        }
    }

    if let Ok(parsed) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let midnight = parsed
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow!("Invalid date provided"))?;
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(midnight, Utc));
    }

    Err(anyhow!(
        "Invalid datetime format '{}'. Use RFC3339, YYYY-MM-DD HH:MM[:SS], or YYYY-MM-DD",
        value
    ))
}

fn parse_filter_datetime(value: &str, bound: FilterBound) -> Result<DateTime<Utc>> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Ok(parsed.with_timezone(&Utc));
    }

    for format in [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%dT%H:%M",
    ] {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(value, format) {
            return Ok(DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc));
        }
    }

    if let Ok(parsed) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let datetime = match bound {
            FilterBound::Start => parsed
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| anyhow!("Invalid start date provided"))?,
            FilterBound::End => parsed
                .and_hms_nano_opt(23, 59, 59, 999_999_999)
                .ok_or_else(|| anyhow!("Invalid end date provided"))?,
        };
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc));
    }

    Err(anyhow!(
        "Invalid filter datetime format '{}'. Use RFC3339, YYYY-MM-DD HH:MM[:SS], or YYYY-MM-DD",
        value
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        AnalyticsTimeFilter, JournalAnalytics, calculate_calendar_analytics,
        calculate_journal_analytics, filter_entries_by_time,
    };
    use crate::service::turso::schema::tables::journal_table::JournalEntry;
    use chrono::{DateTime, Utc};

    fn sample_entry(
        symbol: &str,
        symbol_name: &str,
        account_id: &str,
        position_size: f64,
        total_pl: f64,
        risk_reward: f64,
        close_date: &str,
    ) -> JournalEntry {
        JournalEntry {
            id: format!("{symbol}-{position_size}"),
            user_id: "user_1".to_string(),
            account_id: account_id.to_string(),
            reviewed: true,
            open_date: "2026-03-01T09:00:00Z".to_string(),
            close_date: close_date.to_string(),
            entry_price: 100.0,
            exit_price: 110.0,
            position_size,
            symbol: symbol.to_string(),
            symbol_name: symbol_name.to_string(),
            status: if total_pl < 0.0 {
                "loss".to_string()
            } else {
                "profit".to_string()
            },
            total_pl,
            net_roi: total_pl,
            duration: 3600,
            stop_loss: 95.0,
            risk_reward,
            trade_type: "long".to_string(),
            mistakes: "none".to_string(),
            entry_tactics: "breakout".to_string(),
            edges_spotted: "trend".to_string(),
            notes: None,
        }
    }

    #[test]
    fn returns_zeroed_analytics_for_empty_entries() {
        let analytics = calculate_journal_analytics(Vec::new());

        assert_eq!(
            analytics,
            JournalAnalytics {
                win_rate: 0.0,
                cumulative_profit: 0.0,
                average_risk_to_reward: 0.0,
                average_gain: 0.0,
                average_loss: 0.0,
                profit_factor: None,
                biggest_win: None,
                biggest_loss: None,
            }
        );
    }

    #[test]
    fn calculates_expected_trade_metrics() {
        let analytics = calculate_journal_analytics(vec![
            sample_entry(
                "AAPL",
                "Apple",
                "acc_1",
                1_000.0,
                10.0,
                2.0,
                "2026-03-01T10:00:00Z",
            ),
            sample_entry(
                "TSLA",
                "Tesla",
                "acc_1",
                500.0,
                -8.0,
                1.0,
                "2026-03-02T10:00:00Z",
            ),
            sample_entry(
                "NVDA",
                "NVIDIA",
                "acc_1",
                1_500.0,
                4.0,
                3.0,
                "2026-03-03T10:00:00Z",
            ),
        ]);

        assert_eq!(analytics.win_rate, 66.66666666666666);
        assert_eq!(analytics.cumulative_profit, 120.0);
        assert_eq!(analytics.average_risk_to_reward, 2.0);
        assert_eq!(analytics.average_gain, 80.0);
        assert_eq!(analytics.average_loss, 40.0);
        assert_eq!(analytics.profit_factor, Some(4.0));
        assert_eq!(
            analytics
                .biggest_win
                .as_ref()
                .map(|trade| trade.symbol.as_str()),
            Some("AAPL")
        );
        assert_eq!(
            analytics
                .biggest_loss
                .as_ref()
                .map(|trade| trade.symbol.as_str()),
            Some("TSLA")
        );
        assert_eq!(
            analytics.biggest_loss.as_ref().map(|trade| trade.amount),
            Some(-40.0)
        );
    }

    #[test]
    fn omits_profit_factor_when_there_are_no_losses() {
        let analytics = calculate_journal_analytics(vec![
            sample_entry(
                "MSFT",
                "Microsoft",
                "acc_1",
                1_000.0,
                5.0,
                1.5,
                "2026-03-01T10:00:00Z",
            ),
            sample_entry(
                "META",
                "Meta",
                "acc_1",
                750.0,
                4.0,
                2.5,
                "2026-03-02T10:00:00Z",
            ),
        ]);

        assert_eq!(analytics.profit_factor, None);
        assert_eq!(analytics.biggest_loss, None);
    }

    #[test]
    fn filters_entries_for_last_7_days() {
        let now = parse_utc("2026-03-14T12:00:00Z");
        let entries = vec![
            sample_entry(
                "AAPL",
                "Apple",
                "acc_1",
                1_000.0,
                10.0,
                2.0,
                "2026-03-13T10:00:00Z",
            ),
            sample_entry(
                "TSLA",
                "Tesla",
                "acc_1",
                500.0,
                -8.0,
                1.0,
                "2026-03-01T10:00:00Z",
            ),
        ];

        let filtered =
            filter_entries_by_time(entries, &AnalyticsTimeFilter::Last7Days, now).unwrap();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].symbol, "AAPL");
    }

    #[test]
    fn filters_entries_for_year_to_date() {
        let now = parse_utc("2026-03-14T12:00:00Z");
        let entries = vec![
            sample_entry(
                "AAPL",
                "Apple",
                "acc_1",
                1_000.0,
                10.0,
                2.0,
                "2026-02-10T10:00:00Z",
            ),
            sample_entry(
                "TSLA",
                "Tesla",
                "acc_1",
                500.0,
                -8.0,
                1.0,
                "2025-12-31T10:00:00Z",
            ),
        ];

        let filtered =
            filter_entries_by_time(entries, &AnalyticsTimeFilter::YearToDate, now).unwrap();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].symbol, "AAPL");
    }

    #[test]
    fn includes_future_dated_entries_within_one_year() {
        // Entries with close_date in the future (e.g. a trade logged ahead of time)
        // should still be included in range filters
        let now = parse_utc("2026-03-14T08:00:00Z");
        let entries = vec![
            sample_entry(
                "AAPL",
                "Apple",
                "acc_1",
                1_000.0,
                10.0,
                2.0,
                "2026-03-18T08:05:00Z", // 4 days in the future — should be included
            ),
            sample_entry(
                "TSLA",
                "Tesla",
                "acc_1",
                500.0,
                -8.0,
                1.0,
                "2025-02-01T10:00:00Z", // outside 30 day window — should be excluded
            ),
        ];

        let filtered =
            filter_entries_by_time(entries, &AnalyticsTimeFilter::Last30Days, now).unwrap();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].symbol, "AAPL");
    }

    #[test]
    fn filters_entries_for_custom_date_range_inclusively() {
        let now = parse_utc("2026-03-14T12:00:00Z");
        let entries = vec![
            sample_entry(
                "AAPL",
                "Apple",
                "acc_1",
                1_000.0,
                10.0,
                2.0,
                "2026-03-10T10:00:00Z",
            ),
            sample_entry(
                "TSLA",
                "Tesla",
                "acc_1",
                500.0,
                -8.0,
                1.0,
                "2026-03-14T22:00:00Z",
            ),
            sample_entry(
                "NVDA",
                "NVIDIA",
                "acc_1",
                1_500.0,
                4.0,
                3.0,
                "2026-03-15T10:00:00Z",
            ),
        ];

        let filtered = filter_entries_by_time(
            entries,
            &AnalyticsTimeFilter::Custom {
                start_date: "2026-03-10".to_string(),
                end_date: "2026-03-14".to_string(),
            },
            now,
        )
        .unwrap();

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].symbol, "AAPL");
        assert_eq!(filtered[1].symbol, "TSLA");
    }

    #[test]
    fn rejects_invalid_custom_date_range() {
        let now = parse_utc("2026-03-14T12:00:00Z");
        let entries = vec![sample_entry(
            "AAPL",
            "Apple",
            "acc_1",
            1_000.0,
            10.0,
            2.0,
            "2026-03-13T10:00:00Z",
        )];

        let error = filter_entries_by_time(
            entries,
            &AnalyticsTimeFilter::Custom {
                start_date: "2026-03-14".to_string(),
                end_date: "2026-03-10".to_string(),
            },
            now,
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("custom end_date must be on or after start_date")
        );
    }

    #[test]
    fn builds_calendar_analytics_for_a_month() {
        let calendar = calculate_calendar_analytics(
            vec![
                sample_entry(
                    "AAPL",
                    "Apple",
                    "acc_1",
                    1_000.0,
                    10.0,
                    2.0,
                    "2026-04-04T10:00:00Z",
                ),
                sample_entry(
                    "TSLA",
                    "Tesla",
                    "acc_1",
                    500.0,
                    -20.0,
                    1.0,
                    "2026-04-04T14:00:00Z",
                ),
                sample_entry(
                    "NVDA",
                    "NVIDIA",
                    "acc_1",
                    2_000.0,
                    5.0,
                    3.0,
                    "2026-04-09T10:00:00Z",
                ),
                sample_entry(
                    "META",
                    "Meta",
                    "acc_1",
                    800.0,
                    -10.0,
                    1.5,
                    "2026-04-17T10:00:00Z",
                ),
                sample_entry(
                    "MSFT",
                    "Microsoft",
                    "acc_1",
                    1_000.0,
                    8.0,
                    2.5,
                    "2026-05-01T10:00:00Z",
                ),
            ],
            2026,
            4,
        )
        .unwrap();

        assert_eq!(calendar.year, 2026);
        assert_eq!(calendar.month, 4);
        assert_eq!(calendar.month_profit, 20.0);
        assert_eq!(calendar.trade_count, 4);
        assert_eq!(calendar.trading_days, 3);
        assert_eq!(calendar.grid_start, "2026-03-29");
        assert_eq!(calendar.grid_end, "2026-05-02");
        assert_eq!(calendar.days.len(), 30);
        assert_eq!(calendar.weeks.len(), 5);

        let april_4 = calendar
            .days
            .iter()
            .find(|day| day.date == "2026-04-04")
            .unwrap();
        assert_eq!(april_4.profit, 0.0);
        assert_eq!(april_4.trade_count, 2);
        assert_eq!(april_4.win_rate, 50.0);

        let april_9 = calendar
            .days
            .iter()
            .find(|day| day.date == "2026-04-09")
            .unwrap();
        assert_eq!(april_9.profit, 100.0);
        assert_eq!(april_9.trade_count, 1);

        assert_eq!(
            calendar.weeks[0],
            super::CalendarWeekSummary {
                week_index: 1,
                week_start: "2026-03-29".to_string(),
                week_end: "2026-04-04".to_string(),
                profit: 0.0,
                trade_count: 2,
                trading_days: 1,
            }
        );
        assert_eq!(calendar.weeks[1].profit, 100.0);
        assert_eq!(calendar.weeks[2].profit, -80.0);
    }

    #[test]
    fn rejects_invalid_calendar_month() {
        let error = calculate_calendar_analytics(Vec::new(), 2026, 13).unwrap_err();
        assert!(error.to_string().contains("month must be between 1 and 12"));
    }

    fn parse_utc(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }
}
