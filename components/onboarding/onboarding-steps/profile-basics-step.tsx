"use client";

import { useState, useEffect } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ProfilePictureUpload } from '../profile-picture-upload';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

interface ProfileBasicsStepProps {
  data: {
    nickname: string;
    timezone: string;
    currency: string;
    profilePictureUuid: string | null;
  };
  onChange: (data: Partial<ProfileBasicsStepProps['data']>) => void;
}

const CURRENCIES = [
  { value: 'USD', label: 'US Dollar (USD)' },
  { value: 'EUR', label: 'Euro (EUR)' },
  { value: 'GBP', label: 'British Pound (GBP)' },
  { value: 'JPY', label: 'Japanese Yen (JPY)' },
  { value: 'CAD', label: 'Canadian Dollar (CAD)' },
  { value: 'AUD', label: 'Australian Dollar (AUD)' },
];

// Common timezones
const TIMEZONES = [
  { value: 'UTC', label: 'UTC (Coordinated Universal Time)' },
  { value: 'America/New_York', label: 'Eastern Time (US)' },
  { value: 'America/Chicago', label: 'Central Time (US)' },
  { value: 'America/Denver', label: 'Mountain Time (US)' },
  { value: 'America/Los_Angeles', label: 'Pacific Time (US)' },
  { value: 'Europe/London', label: 'London (GMT)' },
  { value: 'Europe/Paris', label: 'Paris (CET)' },
  { value: 'Asia/Tokyo', label: 'Tokyo (JST)' },
  { value: 'Asia/Shanghai', label: 'Shanghai (CST)' },
  { value: 'Australia/Sydney', label: 'Sydney (AEST)' },
];

export function ProfileBasicsStep({ data, onChange }: ProfileBasicsStepProps) {
  const [detectedTimezone, setDetectedTimezone] = useState<string>('UTC');

  useEffect(() => {
    // Auto-detect timezone
    try {
      const tz = Intl.DateTimeFormat().resolvedOptions().timeZone;
      setDetectedTimezone(tz);
      if (!data.timezone || data.timezone === 'UTC') {
        onChange({ timezone: tz });
      }
    } catch {
      // Fallback to UTC
    }
  }, []);

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <Label htmlFor="nickname">Display Name (Nickname) *</Label>
        <Input
          id="nickname"
          placeholder="Enter your nickname"
          value={data.nickname}
          onChange={(e) => onChange({ nickname: e.target.value })}
          required
          maxLength={50}
        />
        <p className="text-xs text-muted-foreground">
          This is how others will see your name
        </p>
      </div>

      <div className="space-y-2">
        <Label htmlFor="timezone">Timezone *</Label>
        <Select
          value={data.timezone || detectedTimezone}
          onValueChange={(value) => onChange({ timezone: value })}
        >
          <SelectTrigger id="timezone">
            <SelectValue placeholder="Select timezone" />
          </SelectTrigger>
          <SelectContent>
            {TIMEZONES.map((tz) => (
              <SelectItem key={tz.value} value={tz.value}>
                {tz.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          Auto-detected: {detectedTimezone}
        </p>
      </div>

      <div className="space-y-2">
        <Label htmlFor="currency">Currency *</Label>
        <Select
          value={data.currency || 'USD'}
          onValueChange={(value) => onChange({ currency: value })}
        >
          <SelectTrigger id="currency">
            <SelectValue placeholder="Select currency" />
          </SelectTrigger>
          <SelectContent>
            {CURRENCIES.map((curr) => (
              <SelectItem key={curr.value} value={curr.value}>
                {curr.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <ProfilePictureUpload
        value={data.profilePictureUuid || undefined}
        onChange={(uuid) => onChange({ profilePictureUuid: uuid || null })}
      />
    </div>
  );
}
