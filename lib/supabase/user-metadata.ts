"use client";

import { createClient } from "@/lib/supabase/client";

export interface OnboardingState {
  hasSeenTour: boolean;
}

export async function getUserOnboardingState(): Promise<OnboardingState> {
  const supabase = createClient();
  const { data: { user }, error } = await supabase.auth.getUser();
  if (error || !user) {
    return { hasSeenTour: false };
  }
  const hasSeenTour = Boolean((user.user_metadata as Record<string, unknown> | null)?.hasSeenTour);
  return { hasSeenTour };
}

export async function setUserOnboardingCompleted(): Promise<void> {
  const supabase = createClient();
  const { data: { user } } = await supabase.auth.getUser();
  if (!user) return;
  const current = (user.user_metadata as Record<string, unknown> | null) || {};
  await supabase.auth.updateUser({
    data: { ...current, hasSeenTour: true },
  });
  try {
    if (typeof window !== 'undefined') {
      localStorage.setItem('hasSeenTour', 'true');
    }
  } catch {}
}

export function getLocalHasSeenTour(): boolean {
  try {
    if (typeof window === 'undefined') return false;
    return localStorage.getItem('hasSeenTour') === 'true';
  } catch {
    return false;
  }
}

