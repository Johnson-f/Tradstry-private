"use client";

import { useAuth } from "@clerk/nextjs";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useGraphQL } from "@/lib/client";
import * as journalService from "@/lib/service/journal";
import type {
  CreateJournalEntryInput,
  JournalEntry,
  UpdateJournalEntryInput,
} from "@/lib/types/journal";

const JOURNAL_KEY = ["journal"] as const;

export function useJournalEntries() {
  const { isLoaded, isSignedIn } = useAuth();
  const fetcher = useGraphQL();

  return useQuery<JournalEntry[]>({
    queryKey: JOURNAL_KEY,
    queryFn: () => journalService.fetchJournalEntries(fetcher),
    enabled: isLoaded && isSignedIn,
  });
}

export function useJournalEntriesForAccount(accountId: string | null) {
  const { isLoaded, isSignedIn } = useAuth();
  const fetcher = useGraphQL();

  return useQuery<JournalEntry[]>({
    queryKey: [...JOURNAL_KEY, "account", accountId],
    queryFn: async () => {
      const entries = await journalService.fetchJournalEntries(fetcher);
      if (!accountId) {
        return [];
      }

      return entries.filter((entry) => entry.accountId === accountId);
    },
    enabled: isLoaded && isSignedIn,
  });
}

export function useJournalEntry(id: string | null) {
  const { isLoaded, isSignedIn } = useAuth();
  const fetcher = useGraphQL();

  return useQuery<JournalEntry | null>({
    queryKey: [...JOURNAL_KEY, id],
    queryFn: () => {
      if (!id) {
        throw new Error("journal entry id is required");
      }
      return journalService.fetchJournalEntry(fetcher, id);
    },
    enabled: isLoaded && isSignedIn && !!id,
  });
}

export function useCreateJournalEntry() {
  const fetcher = useGraphQL();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: CreateJournalEntryInput) =>
      journalService.createJournalEntry(fetcher, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: JOURNAL_KEY });
    },
  });
}

export function useUpdateJournalEntry() {
  const fetcher = useGraphQL();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      id,
      input,
    }: {
      id: string;
      input: UpdateJournalEntryInput;
    }) => journalService.updateJournalEntry(fetcher, id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: JOURNAL_KEY });
    },
  });
}

export function useDeleteJournalEntry() {
  const fetcher = useGraphQL();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => journalService.deleteJournalEntry(fetcher, id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: JOURNAL_KEY });
    },
  });
}
