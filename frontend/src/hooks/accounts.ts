"use client";

import { useAuth } from "@clerk/nextjs";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useGraphQL } from "@/lib/client";
import type {
  Account,
  CreateAccountInput,
  UpdateAccountInput,
} from "@/lib/types/accounts";
import * as accountService from "@/lib/service/accounts";

const ACCOUNTS_KEY = ["accounts"] as const;

export function useAccounts() {
  const { isLoaded, isSignedIn } = useAuth();
  const fetcher = useGraphQL();

  return useQuery<Account[]>({
    queryKey: ACCOUNTS_KEY,
    queryFn: () => accountService.fetchAccounts(fetcher),
    enabled: isLoaded && isSignedIn,
  });
}

export function useAccount(id: string | null) {
  const { isLoaded, isSignedIn } = useAuth();
  const fetcher = useGraphQL();

  return useQuery<Account | null>({
    queryKey: [...ACCOUNTS_KEY, id],
    queryFn: () => accountService.fetchAccount(fetcher, id!),
    enabled: isLoaded && isSignedIn && !!id,
  });
}

export function useCreateAccount() {
  const fetcher = useGraphQL();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: CreateAccountInput) =>
      accountService.createAccount(fetcher, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ACCOUNTS_KEY });
    },
  });
}

export function useUpdateAccount() {
  const fetcher = useGraphQL();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateAccountInput }) =>
      accountService.updateAccount(fetcher, id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ACCOUNTS_KEY });
    },
  });
}

export function useDeleteAccount() {
  const fetcher = useGraphQL();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => accountService.deleteAccount(fetcher, id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ACCOUNTS_KEY });
    },
  });
}
