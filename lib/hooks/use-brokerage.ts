'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { brokerageService } from '@/lib/services/brokerage-service';
import type {
  ConnectBrokerageRequest,
  GetTransactionsQuery,
  GetHoldingsQuery,
  GetAccountTransactionsQuery,
  ResolveUnmatchedRequest,
  SyncSummary,
  UseBrokerageConnectionsReturn,
  UseBrokerageAccountsReturn,
  UseBrokerageTransactionsReturn,
  UseBrokerageHoldingsReturn,
  UseBrokerageReturn,
} from '@/lib/types/brokerage';

// Query keys
const QUERY_KEYS = {
  connections: ['brokerage', 'connections'] as const,
  connectionStatus: (id: string) => ['brokerage', 'connection-status', id] as const,
  accounts: ['brokerage', 'accounts'] as const,
  transactions: (query?: GetTransactionsQuery) => ['brokerage', 'transactions', query] as const,
  holdings: (query?: GetHoldingsQuery) => ['brokerage', 'holdings', query] as const,
} as const;

/**
 * Hook to fetch brokerage connections
 */
export function useBrokerageConnections(): UseBrokerageConnectionsReturn {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: QUERY_KEYS.connections,
    queryFn: () => brokerageService.listConnections(),
    staleTime: 2 * 60 * 1000, // 2 minutes
    gcTime: 5 * 60 * 1000, // 5 minutes
    retry: 2,
  });

  return {
    connections: data ?? [],
    loading: isLoading,
    error: error as Error | null,
    refetch: () => {
      void refetch();
    },
  };
}

/**
 * Hook to fetch brokerage accounts
 */
export function useBrokerageAccounts(): UseBrokerageAccountsReturn {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: QUERY_KEYS.accounts,
    queryFn: () => brokerageService.listAccounts(),
    staleTime: 2 * 60 * 1000, // 2 minutes
    gcTime: 5 * 60 * 1000, // 5 minutes
    retry: 2,
  });

  return {
    accounts: data ?? [],
    loading: isLoading,
    error: error as Error | null,
    refetch: () => {
      void refetch();
    },
  };
}

/**
 * Hook to fetch brokerage transactions
 */
export function useBrokerageTransactions(
  query?: GetTransactionsQuery
): UseBrokerageTransactionsReturn {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: QUERY_KEYS.transactions(query),
    queryFn: () => brokerageService.getTransactions(query),
    staleTime: 1 * 60 * 1000, // 1 minute
    gcTime: 5 * 60 * 1000, // 5 minutes
    retry: 2,
  });

  return {
    transactions: data ?? [],
    loading: isLoading,
    error: error as Error | null,
    refetch: () => {
      void refetch();
    },
  };
}

/**
 * Hook to fetch brokerage holdings
 */
export function useBrokerageHoldings(
  query?: GetHoldingsQuery
): UseBrokerageHoldingsReturn {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: QUERY_KEYS.holdings(query),
    queryFn: () => brokerageService.getHoldings(query),
    staleTime: 1 * 60 * 1000, // 1 minute
    gcTime: 5 * 60 * 1000, // 5 minutes
    retry: 2,
  });

  return {
    holdings: data ?? [],
    loading: isLoading,
    error: error as Error | null,
    refetch: () => {
      void refetch();
    },
  };
}

/**
 * Main brokerage hook that combines all brokerage functionality
 */
export function useBrokerage(): UseBrokerageReturn {
  const queryClient = useQueryClient();

  // Connections
  const {
    data: connections,
    isLoading: connectionsLoading,
    error: connectionsError,
    refetch: refetchConnections,
  } = useQuery({
    queryKey: QUERY_KEYS.connections,
    queryFn: () => brokerageService.listConnections(),
    staleTime: 2 * 60 * 1000,
    gcTime: 5 * 60 * 1000,
    retry: 2,
  });

  // Initiate connection mutation
  const initiateMutation = useMutation({
    mutationFn: (request: ConnectBrokerageRequest) =>
      brokerageService.initiateConnection(request),
    onSuccess: () => {
      toast.success('Connection initiated successfully');
      void queryClient.invalidateQueries({ queryKey: QUERY_KEYS.connections });
    },
    onError: (error: Error) => {
      toast.error(`Failed to initiate connection: ${error.message}`);
    },
  });

  // Get connection status mutation
  const statusMutation = useMutation({
    mutationFn: (connectionId: string) =>
      brokerageService.getConnectionStatus(connectionId),
    onError: (error: Error) => {
      toast.error(`Failed to get connection status: ${error.message}`);
    },
  });

  // Delete connection mutation
  const deleteMutation = useMutation({
    mutationFn: (connectionId: string) =>
      brokerageService.deleteConnection(connectionId),
    onSuccess: () => {
      toast.success('Connection deleted successfully');
      void queryClient.invalidateQueries({ queryKey: QUERY_KEYS.connections });
      void queryClient.invalidateQueries({ queryKey: QUERY_KEYS.accounts });
    },
    onError: (error: Error) => {
      toast.error(`Failed to delete connection: ${error.message}`);
    },
  });

  // Complete connection sync mutation
  const completeSyncMutation = useMutation({
    mutationFn: (connectionId: string) =>
      brokerageService.completeConnectionSync(connectionId),
    onSuccess: (data: SyncSummary) => {
      toast.success(
        `Sync completed! ${data.accounts_synced} accounts, ${data.holdings_synced} holdings, ${data.transactions_synced} transactions`
      );
      void queryClient.invalidateQueries({ queryKey: QUERY_KEYS.connections });
      void queryClient.invalidateQueries({ queryKey: QUERY_KEYS.accounts });
      void queryClient.invalidateQueries({ queryKey: ['brokerage', 'transactions'] });
      void queryClient.invalidateQueries({ queryKey: ['brokerage', 'holdings'] });
    },
    onError: (error: Error) => {
      toast.error(`Failed to complete sync: ${error.message}`);
    },
  });

  // Accounts
  const {
    data: accounts,
    isLoading: accountsLoading,
    error: accountsError,
    refetch: refetchAccounts,
  } = useQuery({
    queryKey: QUERY_KEYS.accounts,
    queryFn: () => brokerageService.listAccounts(),
    staleTime: 2 * 60 * 1000,
    gcTime: 5 * 60 * 1000,
    retry: 2,
  });

  // Sync accounts mutation
  const syncMutation = useMutation({
    mutationFn: () => brokerageService.syncAccounts(),
    onSuccess: (data: SyncSummary) => {
      toast.success(
        `Synced ${data.accounts_synced} accounts, ${data.holdings_synced} holdings, ${data.transactions_synced} transactions`
      );
      void queryClient.invalidateQueries({ queryKey: QUERY_KEYS.accounts });
      void queryClient.invalidateQueries({ queryKey: QUERY_KEYS.connections });
      void queryClient.invalidateQueries({ queryKey: ['brokerage', 'transactions'] });
      void queryClient.invalidateQueries({ queryKey: ['brokerage', 'holdings'] });
    },
    onError: (error: Error) => {
      toast.error(`Failed to sync accounts: ${error.message}`);
    },
  });

  // Transactions
  const {
    data: transactions,
    isLoading: transactionsLoading,
    error: transactionsError,
  } = useQuery({
    queryKey: QUERY_KEYS.transactions(),
    queryFn: () => brokerageService.getTransactions(),
    staleTime: 1 * 60 * 1000,
    gcTime: 5 * 60 * 1000,
    retry: 2,
  });

  // Holdings
  const {
    data: holdings,
    isLoading: holdingsLoading,
    error: holdingsError,
  } = useQuery({
    queryKey: QUERY_KEYS.holdings(),
    queryFn: () => brokerageService.getHoldings(),
    staleTime: 1 * 60 * 1000,
    gcTime: 5 * 60 * 1000,
    retry: 2,
  });

  // Account detail mutation
  const accountDetailMutation = useMutation({
    mutationFn: (accountId: string) => brokerageService.getAccountDetail(accountId),
    onError: (error: Error) => {
      toast.error(`Failed to get account detail: ${error.message}`);
    },
  });

  // Account positions mutation
  const accountPositionsMutation = useMutation({
    mutationFn: (accountId: string) => brokerageService.getAccountPositions(accountId),
    onError: (error: Error) => {
      toast.error(`Failed to get account positions: ${error.message}`);
    },
  });

  // Account option positions mutation
  const accountOptionPositionsMutation = useMutation({
    mutationFn: (accountId: string) => brokerageService.getAccountOptionPositions(accountId),
    onError: (error: Error) => {
      toast.error(`Failed to get account option positions: ${error.message}`);
    },
  });

  // Account transactions mutation
  const accountTransactionsMutation = useMutation({
    mutationFn: ({ accountId, query }: { accountId: string; query?: GetAccountTransactionsQuery }) =>
      brokerageService.getAccountTransactions(accountId, query),
    onError: (error: Error) => {
      toast.error(`Failed to get account transactions: ${error.message}`);
    },
  });

  // Unmatched transactions
  const {
    data: unmatchedTransactions,
    isLoading: unmatchedTransactionsLoading,
    error: unmatchedTransactionsError,
    refetch: refetchUnmatchedTransactions,
  } = useQuery({
    queryKey: ['brokerage', 'unmatched-transactions'],
    queryFn: () => brokerageService.getUnmatchedTransactions(),
    staleTime: 1 * 60 * 1000,
    gcTime: 5 * 60 * 1000,
    retry: 2,
  });

  // Resolve unmatched transaction mutation
  const resolveUnmatchedMutation = useMutation({
    mutationFn: ({ id, request }: { id: string; request: ResolveUnmatchedRequest }) =>
      brokerageService.resolveUnmatchedTransaction(id, request),
    onSuccess: () => {
      toast.success('Transaction resolved successfully');
      void queryClient.invalidateQueries({ queryKey: ['brokerage', 'unmatched-transactions'] });
      void queryClient.invalidateQueries({ queryKey: QUERY_KEYS.accounts });
      void queryClient.invalidateQueries({ queryKey: ['brokerage', 'transactions'] });
    },
    onError: (error: Error) => {
      toast.error(`Failed to resolve transaction: ${error.message}`);
    },
  });

  // Ignore unmatched transaction mutation
  const ignoreUnmatchedMutation = useMutation({
    mutationFn: (id: string) => brokerageService.ignoreUnmatchedTransaction(id),
    onSuccess: () => {
      toast.success('Transaction ignored');
      void queryClient.invalidateQueries({ queryKey: ['brokerage', 'unmatched-transactions'] });
    },
    onError: (error: Error) => {
      toast.error(`Failed to ignore transaction: ${error.message}`);
    },
  });

  // Get unmatched suggestions mutation
  const unmatchedSuggestionsMutation = useMutation({
    mutationFn: (id: string) => brokerageService.getUnmatchedSuggestions(id),
    onError: (error: Error) => {
      toast.error(`Failed to get suggestions: ${error.message}`);
    },
  });

  return {
    // Connections
    connections: connections ?? [],
    connectionsLoading,
    connectionsError: connectionsError as Error | null,
    refetchConnections: () => {
      void refetchConnections();
    },

    // Initiate connection
    initiateConnection: async (request: ConnectBrokerageRequest) => {
      return await initiateMutation.mutateAsync(request);
    },
    initiating: initiateMutation.isPending,

    // Connection status
    getConnectionStatus: async (connectionId: string) => {
      return await statusMutation.mutateAsync(connectionId);
    },
    statusLoading: statusMutation.isPending,

    // Delete connection
    deleteConnection: async (connectionId: string) => {
      await deleteMutation.mutateAsync(connectionId);
    },
    deleting: deleteMutation.isPending,

    // Complete connection sync
    completeConnectionSync: async (connectionId: string) => {
      return await completeSyncMutation.mutateAsync(connectionId);
    },
    completingSync: completeSyncMutation.isPending,

    // Accounts
    accounts: accounts ?? [],
    accountsLoading,
    accountsError: accountsError as Error | null,
    refetchAccounts: () => {
      void refetchAccounts();
    },

    // Sync accounts
    syncAccounts: async () => {
      return await syncMutation.mutateAsync();
    },
    syncing: syncMutation.isPending,

    // Transactions
    transactions: transactions ?? [],
    transactionsLoading,
    transactionsError: transactionsError as Error | null,
    refetchTransactions: (query?: GetTransactionsQuery) => {
      void queryClient.invalidateQueries({
        queryKey: QUERY_KEYS.transactions(query),
      });
    },

    // Holdings
    holdings: holdings ?? [],
    holdingsLoading,
    holdingsError: holdingsError as Error | null,
    refetchHoldings: (query?: GetHoldingsQuery) => {
      void queryClient.invalidateQueries({
        queryKey: QUERY_KEYS.holdings(query),
      });
    },

    // Get account detail
    getAccountDetail: async (accountId: string) => {
      return await accountDetailMutation.mutateAsync(accountId);
    },
    accountDetailLoading: accountDetailMutation.isPending,

    // Get account positions
    getAccountPositions: async (accountId: string) => {
      return await accountPositionsMutation.mutateAsync(accountId);
    },
    accountPositionsLoading: accountPositionsMutation.isPending,

    // Get account option positions
    getAccountOptionPositions: async (accountId: string) => {
      return await accountOptionPositionsMutation.mutateAsync(accountId);
    },
    accountOptionPositionsLoading: accountOptionPositionsMutation.isPending,

    // Get account transactions
    getAccountTransactions: async (accountId: string, query?: GetAccountTransactionsQuery) => {
      return await accountTransactionsMutation.mutateAsync({ accountId, query });
    },
    accountTransactionsLoading: accountTransactionsMutation.isPending,

    // Unmatched transactions
    unmatchedTransactions: unmatchedTransactions ?? [],
    unmatchedTransactionsLoading,
    unmatchedTransactionsError: unmatchedTransactionsError as Error | null,
    refetchUnmatchedTransactions: () => {
      void refetchUnmatchedTransactions();
    },

    // Resolve unmatched transaction
    resolveUnmatchedTransaction: async (id: string, request: ResolveUnmatchedRequest) => {
      return await resolveUnmatchedMutation.mutateAsync({ id, request });
    },
    resolvingUnmatched: resolveUnmatchedMutation.isPending,

    // Ignore unmatched transaction
    ignoreUnmatchedTransaction: async (id: string) => {
      await ignoreUnmatchedMutation.mutateAsync(id);
    },
    ignoringUnmatched: ignoreUnmatchedMutation.isPending,

    // Get unmatched suggestions
    getUnmatchedSuggestions: async (id: string) => {
      return await unmatchedSuggestionsMutation.mutateAsync(id);
    },
    unmatchedSuggestionsLoading: unmatchedSuggestionsMutation.isPending,
  };
}

