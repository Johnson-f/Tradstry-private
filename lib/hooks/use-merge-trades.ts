'use client';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { brokerageService } from '@/lib/services/brokerage-service';
import type { MergeTransactionsRequest } from '@/lib/types/brokerage';

/**
 * Hook to merge brokerage transactions into a stock or option trade
 */
export function useMergeTrades() {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: (request: MergeTransactionsRequest) =>
      brokerageService.mergeTransactions(request),
    onSuccess: () => {
      // Invalidate transactions queries to refresh the list
      queryClient.invalidateQueries({ queryKey: ['brokerage', 'transactions'] });
      // Also invalidate stocks and options queries since we're creating trades
      queryClient.invalidateQueries({ queryKey: ['stocks'] });
      queryClient.invalidateQueries({ queryKey: ['options'] });
      toast.success('Transactions merged successfully');
    },
    onError: (error: Error) => {
      toast.error(error.message || 'Failed to merge transactions');
    },
  });

  return {
    mergeTransactions: mutation.mutateAsync,
    isMerging: mutation.isPending,
    error: mutation.error as Error | null,
  };
}

