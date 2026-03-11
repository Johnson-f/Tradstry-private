"use client";

import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

interface TradingExperienceStepProps {
  data: {
    trading_experience_level: string;
    primary_trading_goal: string;
    trading_style: string;
  };
  onChange: (data: Partial<TradingExperienceStepProps['data']>) => void;
}

const EXPERIENCE_LEVELS = [
  { value: 'Beginner', label: 'Beginner' },
  { value: 'Intermediate', label: 'Intermediate' },
  { value: 'Advanced', label: 'Advanced' },
  { value: 'Professional', label: 'Professional' },
];

const TRADING_GOALS = [
  { value: 'track_analyze', label: 'Track and analyze my trades' },
  { value: 'improve_performance', label: 'Improve my trading performance' },
  { value: 'build_journal', label: 'Build a comprehensive trading journal' },
  { value: 'learn_strategies', label: 'Learn trading strategies' },
];

const TRADING_STYLES = [
  { value: 'Day Trading', label: 'Day Trading' },
  { value: 'Swing Trading', label: 'Swing Trading' },
  { value: 'Position Trading', label: 'Position Trading' },
  { value: 'Options Trading', label: 'Options Trading' },
  { value: 'Mixed', label: 'Mixed (Multiple styles)' },
];

export function TradingExperienceStep({ data, onChange }: TradingExperienceStepProps) {
  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <Label htmlFor="experience-level">Trading Experience Level *</Label>
        <Select
          value={data.trading_experience_level}
          onValueChange={(value) => onChange({ trading_experience_level: value })}
        >
          <SelectTrigger id="experience-level">
            <SelectValue placeholder="Select your experience level" />
          </SelectTrigger>
          <SelectContent>
            {EXPERIENCE_LEVELS.map((level) => (
              <SelectItem key={level.value} value={level.value}>
                {level.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="space-y-2">
        <Label htmlFor="trading-goal">Primary Trading Goal *</Label>
        <Select
          value={data.primary_trading_goal}
          onValueChange={(value) => onChange({ primary_trading_goal: value })}
        >
          <SelectTrigger id="trading-goal">
            <SelectValue placeholder="Select your primary goal" />
          </SelectTrigger>
          <SelectContent>
            {TRADING_GOALS.map((goal) => (
              <SelectItem key={goal.value} value={goal.value}>
                {goal.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="space-y-2">
        <Label htmlFor="trading-style">Trading Style *</Label>
        <Select
          value={data.trading_style}
          onValueChange={(value) => onChange({ trading_style: value })}
        >
          <SelectTrigger id="trading-style">
            <SelectValue placeholder="Select your trading style" />
          </SelectTrigger>
          <SelectContent>
            {TRADING_STYLES.map((style) => (
              <SelectItem key={style.value} value={style.value}>
                {style.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </div>
  );
}
