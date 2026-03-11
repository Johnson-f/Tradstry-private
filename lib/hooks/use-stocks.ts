'use client';

import { useEffect } from 'react';
import { useQuery, useQueryClient, useMutation } from '@tanstack/react-query';
import { useWs } from '@/lib/websocket/provider';
import apiConfig, { getFullUrl } from '@/lib/config/api';
import { createClient } from '@/lib/supabase/client';
import type { Stock, CreateStockRequest, UpdateStockRequest } from '@/lib/types/stocks';

async function fetchStocks(params?: { openOnly?: boolean }): Promise<Stock[]> {
  const supabase = createClient();
  const { data: { session } } = await supabase.auth.getSession();
  const token = session?.access_token;

  if (!token) {
    throw new Error('User not authenticated');
  }

  const endpoint = params?.openOnly !== undefined
    ? apiConfig.endpoints.stocks.withQuery({ openOnly: params.openOnly })
    : apiConfig.endpoints.stocks.base;

  const res = await fetch(getFullUrl(endpoint), {
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
    credentials: 'include',
  });

  if (!res.ok) {
    if (res.status === 401) {
      throw new Error('Authentication failed');
    }
    throw new Error('Failed to fetch stocks');
  }

  const json = await res.json();
  return (json.data ?? []) as Stock[];
}

export function useStocks(params?: { openOnly?: boolean }) {
  const queryClient = useQueryClient();
  const { subscribe } = useWs();

  const query = useQuery<Stock[]>({
    queryKey: ['stocks', params],
    queryFn: () => fetchStocks(params),
    staleTime: 5 * 60 * 1000,
  });

  useEffect(() => {
    const unsubCreate = subscribe('stock:created', (data) => {
      const stock = data as Stock;
      queryClient.setQueryData<Stock[]>(['stocks', params], (prev) => prev ? [stock, ...prev] : [stock]);
      // Also update the base query without params
      queryClient.setQueryData<Stock[]>(['stocks'], (prev) => prev ? [stock, ...prev] : [stock]);
    });
    const unsubUpdate = subscribe('stock:updated', (data) => {
      const stock = data as Stock;
      queryClient.setQueryData<Stock[]>(['stocks', params], (prev) => prev?.map(s => s.id === stock.id ? stock : s) ?? [stock]);
      // Also update the base query without params
      queryClient.setQueryData<Stock[]>(['stocks'], (prev) => prev?.map(s => s.id === stock.id ? stock : s) ?? [stock]);
    });
    const unsubDelete = subscribe('stock:deleted', (data) => {
      const payload = data as { id: number };
      queryClient.setQueryData<Stock[]>(['stocks', params], (prev) => prev?.filter(s => s.id !== payload.id) ?? []);
      // Also update the base query without params
      queryClient.setQueryData<Stock[]>(['stocks'], (prev) => prev?.filter(s => s.id !== payload.id) ?? []);
    });
    return () => { unsubCreate(); unsubUpdate(); unsubDelete(); };
  }, [subscribe, queryClient, params]);

  return query;
}


async function authHeader() {
  const supabase = createClient();
  const { data: { session } } = await supabase.auth.getSession();
  const token = session?.access_token;
  if (!token) throw new Error('User not authenticated');
  return { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' } as const;
}

export function useCreateStock() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (payload: CreateStockRequest) => {
      const headers = await authHeader();
      const res = await fetch(getFullUrl(apiConfig.endpoints.stocks.base), {
        method: 'POST', 
        headers, 
        body: JSON.stringify(payload), 
        credentials: 'include',
      });
      
      if (!res.ok) {
        let message = 'Failed to create stock';
        try {
          const err = await res.json();
          message = err?.message || err?.error || message;
        } catch {
          try { 
            const text = await res.text(); 
            if (text) message = text;
          } catch {}
        }
        throw new Error(message);
      }
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['stocks'] });
    },
  });
}

export function useUpdateStock() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, updates }: { id: number; updates: UpdateStockRequest }) => {
      const headers = await authHeader();
      const res = await fetch(getFullUrl(apiConfig.endpoints.stocks.byId(id)), {
        method: 'PUT', headers, body: JSON.stringify(updates), credentials: 'include',
      });
      if (!res.ok) throw new Error('Failed to update stock');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['stocks'] });
    },
  });
}

export function useDeleteStock() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: number) => {
      const headers = await authHeader();
      const res = await fetch(getFullUrl(apiConfig.endpoints.stocks.byId(id)), {
        method: 'DELETE', headers, credentials: 'include',
      });
      if (!res.ok) throw new Error('Failed to delete stock');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['stocks'] });
    },
  });
}

async function fetchStock(id: number): Promise<Stock> {
  const supabase = createClient();
  const { data: { session } } = await supabase.auth.getSession();
  const token = session?.access_token;

  if (!token) {
    throw new Error('User not authenticated');
  }

  const res = await fetch(getFullUrl(apiConfig.endpoints.stocks.byId(id)), {
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
    credentials: 'include',
  });

  if (!res.ok) {
    if (res.status === 401) {
      throw new Error('Authentication failed');
    }
    if (res.status === 404) {
      throw new Error('Stock not found');
    }
    throw new Error('Failed to fetch stock');
  }

  const json = await res.json();
  return (json.data ?? json) as Stock;
}

export function useStock(id: number, enabled: boolean = true) {
  return useQuery<Stock>({
    queryKey: ['stocks', id],
    queryFn: () => fetchStock(id),
    enabled: enabled && id > 0,
    staleTime: 5 * 60 * 1000,
    gcTime: 10 * 60 * 1000,
  });
}


