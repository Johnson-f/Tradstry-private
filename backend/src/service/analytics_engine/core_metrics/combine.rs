use crate::models::analytics::CoreMetrics;

/// Combine metrics from stocks and options tables
pub(crate) fn combine_core_metrics(stocks: CoreMetrics, options: CoreMetrics) -> CoreMetrics {
    let total_trades = stocks.total_trades + options.total_trades;
    let total_winning_trades = stocks.winning_trades + options.winning_trades;
    let total_losing_trades = stocks.losing_trades + options.losing_trades;
    let total_break_even_trades = stocks.break_even_trades + options.break_even_trades;

    let total_pnl = stocks.total_pnl + options.total_pnl;
    let total_gross_profit = stocks.gross_profit + options.gross_profit;
    let total_gross_loss = stocks.gross_loss + options.gross_loss;
    let total_commissions = stocks.total_commissions + options.total_commissions;

    // Calculate combined averages (weighted by trade count)
    let stocks_weight = if total_trades > 0 {
        stocks.total_trades as f64 / total_trades as f64
    } else {
        0.0
    };
    let options_weight = if total_trades > 0 {
        options.total_trades as f64 / total_trades as f64
    } else {
        0.0
    };

    let average_win = if total_winning_trades > 0 {
        (stocks.average_win * stocks.winning_trades as f64
            + options.average_win * options.winning_trades as f64)
            / total_winning_trades as f64
    } else {
        0.0
    };

    let average_loss = if total_losing_trades > 0 {
        (stocks.average_loss * stocks.losing_trades as f64
            + options.average_loss * options.losing_trades as f64)
            / total_losing_trades as f64
    } else {
        0.0
    };

    let average_position_size = stocks.average_position_size * stocks_weight
        + options.average_position_size * options_weight;
    let average_commission_per_trade = if total_trades > 0 {
        total_commissions / total_trades as f64
    } else {
        0.0
    };

    let biggest_winner = stocks.biggest_winner.max(options.biggest_winner);
    let biggest_loser = stocks.biggest_loser.min(options.biggest_loser);

    let win_rate = if total_trades > 0 {
        (total_winning_trades as f64 / total_trades as f64) * 100.0
    } else {
        0.0
    };
    let loss_rate = if total_trades > 0 {
        (total_losing_trades as f64 / total_trades as f64) * 100.0
    } else {
        0.0
    };

    let profit_factor = if total_gross_loss != 0.0 {
        total_gross_profit.abs() / total_gross_loss.abs()
    } else if total_gross_profit > 0.0 {
        f64::INFINITY
    } else {
        0.0
    };

    let win_loss_ratio = if average_loss != 0.0 {
        average_win.abs() / average_loss.abs()
    } else if average_win > 0.0 {
        f64::INFINITY
    } else {
        0.0
    };

    // Take the maximum consecutive streaks from both stocks and options
    let max_consecutive_wins = stocks
        .max_consecutive_wins
        .max(options.max_consecutive_wins);
    let max_consecutive_losses = stocks
        .max_consecutive_losses
        .max(options.max_consecutive_losses);

    CoreMetrics {
        total_trades,
        winning_trades: total_winning_trades,
        losing_trades: total_losing_trades,
        break_even_trades: total_break_even_trades,
        win_rate,
        loss_rate,
        total_pnl,
        net_profit_loss: total_pnl,
        gross_profit: total_gross_profit,
        gross_loss: total_gross_loss,
        average_win,
        average_loss,
        average_position_size,
        biggest_winner,
        biggest_loser,
        profit_factor,
        win_loss_ratio,
        max_consecutive_wins,
        max_consecutive_losses,
        total_commissions,
        average_commission_per_trade,
    }
}
