"use client";

import { useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { tradeTagService } from '@/lib/services/trade-tag-service';
import type {
  CreateTagRequest,
  UpdateTagRequest,
  TagQuery,
} from '@/lib/types/trade-tags';

export function useTradeTags(query?: TagQuery) {
  return useQuery({
    queryKey: ['trade-tags', query?.category ?? 'all', query?.limit ?? 100, query?.offset ?? 0],
    queryFn: async () => {
      const res = await tradeTagService.listTags(query);
      return res.data ?? [];
    },
    staleTime: 60_000,
  });
}

export function useTradeTagCategories() {
  return useQuery({
    queryKey: ['trade-tag-categories'],
    queryFn: async () => {
      const res = await tradeTagService.getCategories();
      return res.data ?? [];
    },
    staleTime: 5 * 60_000,
  });
}

export function useTradeTagCrud() {
  const queryClient = useQueryClient();

  const createTag = useMutation({
    mutationFn: (payload: CreateTagRequest) => tradeTagService.createTag(payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['trade-tags'] });
      queryClient.invalidateQueries({ queryKey: ['trade-tag-categories'] });
    },
  });

  const updateTag = useMutation({
    mutationFn: ({ id, payload }: { id: string; payload: UpdateTagRequest }) => tradeTagService.updateTag(id, payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['trade-tags'] });
      queryClient.invalidateQueries({ queryKey: ['trade-tag-categories'] });
    },
  });

  const deleteTag = useMutation({
    mutationFn: (id: string) => tradeTagService.deleteTag(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['trade-tags'] });
      queryClient.invalidateQueries({ queryKey: ['trade-tag-categories'] });
    },
  });

  return {
    createTag,
    updateTag,
    deleteTag,
  };
}

export function useTradeTagsForTrade(params: { tradeType: 'stock' | 'option'; tradeId: number }) {
  const { tradeType, tradeId } = params;
  const queryKey = useMemo(() => ['trade-tags', tradeType, tradeId], [tradeType, tradeId]);
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey,
    queryFn: async () => {
      const res = tradeType === 'stock'
        ? await tradeTagService.getStockTradeTags(tradeId)
        : await tradeTagService.getOptionTradeTags(tradeId);
      return res.data ?? [];
    },
    enabled: !!tradeId,
    staleTime: 60_000,
  });

  const addTags = useMutation({
    mutationFn: async (tagIds: string[]) => {
      return tradeType === 'stock'
        ? tradeTagService.addTagsToStockTrade(tradeId, tagIds)
        : tradeTagService.addTagsToOptionTrade(tradeId, tagIds);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey });
    },
  });

  const removeTag = useMutation({
    mutationFn: async (tagId: string) => {
      return tradeType === 'stock'
        ? tradeTagService.removeTagFromStockTrade(tradeId, tagId)
        : tradeTagService.removeTagFromOptionTrade(tradeId, tagId);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey });
    },
  });

  return {
    tags: query.data ?? [],
    isLoading: query.isLoading,
    error: query.error,
    refetch: query.refetch,
    addTags: addTags.mutate,
    addTagsAsync: addTags.mutateAsync,
    isAdding: addTags.isPending,
    removeTag: removeTag.mutate,
    removeTagAsync: removeTag.mutateAsync,
    isRemoving: removeTag.isPending,
  };
}


