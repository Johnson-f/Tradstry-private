"use client";

import {
  NotebookPen,
  ChartCandlestick,
  GraduationCap,
  Library,
  PieChart,
  BrainCog,
  Wallet,
  Notebook,
  LayoutDashboard,
  Sparkles
} from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

const features = [
  {
    icon: NotebookPen,
    title: "Trade Journaling",
    description: "Record every trade with detailed notes, screenshots, and emotional state tracking. Build a comprehensive trading history.",
    color: "text-blue-600 dark:text-blue-400",
  },
  {
    icon: PieChart,
    title: "Advanced Analytics",
    description: "Deep dive into your performance with win rates, Sharpe ratios, drawdown analysis, and custom metrics.",
    color: "text-purple-600 dark:text-purple-400",
  },
  {
    icon: ChartCandlestick,
    title: "Market Data",
    description: "Real-time market data, stock quotes, options chains, earnings calendars, and economic indicators.",
    color: "text-green-600 dark:text-green-400",
  },
  {
    icon: Sparkles,
    title: "AI-Powered Reports",
    description: "Get intelligent insights and automated reports that identify patterns and suggest improvements.",
    color: "text-orange-600 dark:text-orange-400",
  },
  {
    icon: Library,
    title: "Trading Playbook",
    description: "Build and maintain your trading strategies. Document setups, rules, and edge cases.",
    color: "text-pink-600 dark:text-pink-400",
  },
  {
    icon: Notebook,
    title: "Rich Notebook",
    description: "Capture ideas, market observations, and trading plans with a powerful note-taking system.",
    color: "text-indigo-600 dark:text-indigo-400",
  },
  {
    icon: BrainCog,
    title: "Mindset Lab",
    description: "Track your psychology, emotional state, and mental performance alongside your trades.",
    color: "text-red-600 dark:text-red-400",
  },
  {
    icon: Wallet,
    title: "Brokerage Integration",
    description: "Connect your brokerage accounts for automatic trade import and portfolio tracking.",
    color: "text-teal-600 dark:text-teal-400",
  },
  {
    icon: GraduationCap,
    title: "Education Hub",
    description: "Access trading courses, tutorials, and resources to continuously improve your skills.",
    color: "text-cyan-600 dark:text-cyan-400",
  },
  {
    icon: LayoutDashboard,
    title: "Unified Dashboard",
    description: "Get a complete overview of your trading performance, portfolio, and key metrics at a glance.",
    color: "text-amber-600 dark:text-amber-400",
  },
];

export function Features() {
  return (
    <section className="relative py-24 px-4 sm:px-6 lg:px-8">
      {/* Subtle gradient accent */}
      <div className="absolute top-0 left-1/2 -translate-x-1/2 -z-10 size-[600px] rounded-full bg-pink-500/5 blur-3xl" />
      
      <div className="container mx-auto max-w-7xl relative z-10">
        <div className="mb-16 text-center">
          <h2 className="mb-4 text-4xl font-bold tracking-tight sm:text-5xl">
            Everything You Need to
            <span className="block mt-2">Trade Smarter</span>
          </h2>
          <p className="mx-auto max-w-2xl text-lg text-muted-foreground">
            A comprehensive platform that combines journaling, analytics, market data, 
            and AI insights to help you become a better trader.
          </p>
        </div>
        
        <ScrollArea className="w-full">
          <div className="grid gap-6 sm:grid-cols-2 lg:grid-cols-3 pb-4">
            {features.map((feature, index) => {
              const Icon = feature.icon;
              return (
                <Card
                  key={index}
                  className="group relative overflow-hidden border-border/50 transition-all hover:border-border hover:shadow-lg"
                >
                  <CardHeader>
                    <div className="mb-4 flex size-12 items-center justify-center rounded-lg bg-muted">
                      <Icon className={cn("size-6", feature.color)} />
                    </div>
                    <CardTitle className="text-xl">{feature.title}</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <CardDescription className="text-base leading-relaxed">
                      {feature.description}
                    </CardDescription>
                  </CardContent>
                </Card>
              );
            })}
          </div>
        </ScrollArea>
      </div>
    </section>
  );
}

