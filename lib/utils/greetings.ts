export function getTimeBasedGreeting() {
  const hour = new Date().getHours();
  const day = new Date().getDay(); // 0 = Sunday, 6 = Saturday
  const isWeekend = day === 0 || day === 6;
  
  const greetings = [
    { min: 5, max: 11, greeting: 'Good morning' },
    { min: 12, max: 16, greeting: 'Good afternoon' },
    { min: 17, max: 20, greeting: 'Good evening' },
    { min: 21, max: 4, greeting: 'Good night' },
  ];

  const casualGreetings = [
    "How's it going?",
    "What's good?",
    "How's your day going?",
    "Ready to analyze some trades?",
    "Let's make some great trades today!",
    "Stay disciplined, stay profitable!",
    "Trade the plan, plan the trade!",
    "How are the markets treating you?",
    "Cut your losses short; let your profits run.",
    "Trade with the trend, not against it.",
    "Focus on the process, not the profits.",
    "Consistency beats intensity in trading.",
    "The market is always right; adapt accordingly.",
    "Patience pays; don't rush the market.",
    "Emotions are your enemy; trade with discipline.",
    "Risk management is the key to longevity.",
    "Keep your trading journal; learn from mistakes.",
    "Small consistent gains compound over time.",
    "Trade what you see, not what you think.",
    "Let winners run, cut losers fast.",
  ];

  const tradingReminders = [
    "Remember to review your trading plan.",
    "Have you reviewed your watchlist today?",
    "Don't forget to set your stop losses.",
    "Keep an eye on your risk management.",
    "Consider taking profits at your target levels.",
    "Stay patient for your setups.",
    "Review your trading journal for patterns.",
    "Stay disciplined with your trading rules.",
    "Plan your trade and trade your plan.",
    "Never risk more than you can afford to lose.",
    "Focus on risk-reward ratios before entering.",
    "Review your trades and learn from them.",
    "Stay calm during market volatility.",
    "Don't overtrade - quality over quantity.",
    "Protect your capital above all else.",
    "Stick to your strategy, avoid FOMO.",
    "Track your performance metrics regularly.",
    "Set clear entry and exit points.",
  ];

  // Market hours awareness (NYSE/NASDAQ: 9:30 AM - 4:00 PM ET)
  // Convert to local time awareness (simplified - assumes ET for now)
  const marketOpenHour = 9; // 9:30 AM
  const marketCloseHour = 16; // 4:00 PM
  const preMarketStart = 4; // 4:00 AM
  const afterHoursEnd = 20; // 8:00 PM
  
  let marketStatus = '';
  if (!isWeekend) {
    if (hour >= marketOpenHour && hour < marketCloseHour) {
      marketStatus = "Markets are open - time to trade!";
    } else if (hour >= preMarketStart && hour < marketOpenHour) {
      marketStatus = "Pre-market is active - prepare your watchlist.";
    } else if (hour >= marketCloseHour && hour < afterHoursEnd) {
      marketStatus = "Markets closed - review your trades and plan for tomorrow.";
    } else {
      marketStatus = "Markets closed - use this time to analyze and plan.";
    }
  } else {
    marketStatus = "Weekend - perfect time to review your strategy and prepare for next week.";
  }

  const timeGreeting = greetings.find(g => 
    (g.min <= g.max && hour >= g.min && hour <= g.max) ||
    (g.min > g.max && (hour >= g.min || hour <= g.max))
  )?.greeting || 'Hello';

  const randomCasual = casualGreetings[Math.floor(Math.random() * casualGreetings.length)];
  const randomReminder = tradingReminders[Math.floor(Math.random() * tradingReminders.length)];

  return {
    timeGreeting,
    casualGreeting: randomCasual,
    tradingReminder: randomReminder,
    marketStatus,
  };
}