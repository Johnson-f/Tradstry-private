/// Calculate consecutive win and loss streaks from a sequence of P&L values
pub(crate) fn calculate_streaks(trades: &[f64]) -> (u32, u32) {
    let mut current_wins = 0;
    let mut current_losses = 0;
    let mut max_wins = 0;
    let mut max_losses = 0;

    for &pnl in trades {
        if pnl > 0.0 {
            // Win
            current_wins += 1;
            current_losses = 0;
            max_wins = max_wins.max(current_wins);
        } else if pnl < 0.0 {
            // Loss
            current_losses += 1;
            current_wins = 0;
            max_losses = max_losses.max(current_losses);
        } else {
            // Break-even - resets streak
            current_wins = 0;
            current_losses = 0;
        }
    }

    (max_wins, max_losses)
}
