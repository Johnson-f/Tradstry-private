"use client";

import {
  useAccounts as useAccountsQuery,
  useCreateAccount,
  useDeleteAccount,
  useUpdateAccount,
} from "@/hooks/accounts";
import type { Account } from "./types";
import { useActiveAccountStore } from "./store";

export function useAccounts(): Account[] {
  const { data } = useAccountsQuery();
  return data ?? [];
}

export function useAccountsLoading(): boolean {
  const { isLoading, isPending } = useAccountsQuery();
  return isLoading || isPending;
}

export function useAccountsError(): string | null {
  const { error } = useAccountsQuery();
  return error instanceof Error ? error.message : null;
}

export function useActiveAccount(): Account | null {
  const accounts = useAccounts();
  const activeAccountId = useActiveAccountStore((s) => s.activeAccountId);

  if (accounts.length === 0) return null;

  const found = accounts.find((a) => a.id === activeAccountId);
  // If stored ID is stale or null, fall back to first account
  return found ?? accounts[0] ?? null;
}

export function useAccountActions() {
  const createMutation = useCreateAccount();
  const updateMutation = useUpdateAccount();
  const deleteMutation = useDeleteAccount();
  const store = useActiveAccountStore();

  return {
    setActive: store.setActiveAccountId,

    create: (data: {
      name: string;
      icon: string;
      currency: string;
      broker: string | null;
      riskProfile: string;
    }) => {
      createMutation.mutate(data, {
        onSuccess: (account) => {
          store.setActiveAccountId(account.id);
        },
      });
    },

    update: (
      id: string,
      data: {
        name?: string;
        icon?: string;
        currency?: string;
        broker?: string | null;
        riskProfile?: string;
      },
    ) => {
      updateMutation.mutate({ id, input: data });
    },

    delete: (id: string, accounts: Account[]) => {
      const remaining = accounts.filter((a) => a.id !== id);
      store.clearIfDeleted(id, remaining[0]?.id ?? null);
      deleteMutation.mutate(id);
    },
  };
}
