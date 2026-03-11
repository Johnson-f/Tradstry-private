'use client';

import { useEffect } from 'react';
import { useQuery, useQueryClient, useMutation } from '@tanstack/react-query';
import { useWs } from '@/lib/websocket/provider';
import apiConfig, { getFullUrl } from '@/lib/config/api';
import { createClient } from '@/lib/supabase/client';
import type { OptionTrade, CreateOptionRequest, UpdateOptionRequest } from '@/lib/types/options';

async function fetchOptions(params?: { openOnly?: boolean }): Promise<OptionTrade[]> {
  const supabase = createClient();
  const { data: { session } } = await supabase.auth.getSession();
  const token = session?.access_token;

  if (!token) {
    throw new Error('User not authenticated');
  }

  const endpoint = params?.openOnly !== undefined
    ? apiConfig.endpoints.options.withQuery({ openOnly: params.openOnly })
    : apiConfig.endpoints.options.base;

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
    throw new Error('Failed to fetch options');
  }

  const json = await res.json();
  return (json.data ?? []) as OptionTrade[];
}

export function useOptions(params?: { openOnly?: boolean }) {
  const queryClient = useQueryClient();
  const { subscribe } = useWs();

  const query = useQuery<OptionTrade[]>({
    queryKey: ['options', params],
    queryFn: () => fetchOptions(params),
    staleTime: 5 * 60 * 1000,
  });

  useEffect(() => {
    const unsubCreate = subscribe('option:created', (data) => {
      const option = data as OptionTrade;
      queryClient.setQueryData<OptionTrade[]>(['options', params], (prev) => prev ? [option, ...prev] : [option]);
      // Also update the base query without params
      queryClient.setQueryData<OptionTrade[]>(['options'], (prev) => prev ? [option, ...prev] : [option]);
    });
    const unsubUpdate = subscribe('option:updated', (data) => {
      const option = data as OptionTrade;
      queryClient.setQueryData<OptionTrade[]>(['options', params], (prev) => prev?.map(o => o.id === option.id ? option : o) ?? [option]);
      // Also update the base query without params
      queryClient.setQueryData<OptionTrade[]>(['options'], (prev) => prev?.map(o => o.id === option.id ? option : o) ?? [option]);
    });
    const unsubDelete = subscribe('option:deleted', (data) => {
      const payload = data as { id: number };
      queryClient.setQueryData<OptionTrade[]>(['options', params], (prev) => prev?.filter(o => o.id !== payload.id) ?? []);
      // Also update the base query without params
      queryClient.setQueryData<OptionTrade[]>(['options'], (prev) => prev?.filter(o => o.id !== payload.id) ?? []);
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

export function useCreateOption() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (payload: CreateOptionRequest) => {
      const headers = await authHeader();
      const res = await fetch(getFullUrl(apiConfig.endpoints.options.base), {
        method: 'POST', headers, body: JSON.stringify(payload), credentials: 'include',
      });
      if (!res.ok) throw new Error('Failed to create option');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['options'] });
    },
  });
}

export function useUpdateOption() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, updates }: { id: number; updates: UpdateOptionRequest }) => {
      const headers = await authHeader();
      const res = await fetch(getFullUrl(apiConfig.endpoints.options.byId(id)), {
        method: 'PUT', headers, body: JSON.stringify(updates), credentials: 'include',
      });
      if (!res.ok) throw new Error('Failed to update option');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['options'] });
    },
  });
}

export function useDeleteOption() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: number) => {
      const headers = await authHeader();
      const res = await fetch(getFullUrl(apiConfig.endpoints.options.byId(id)), {
        method: 'DELETE', headers, credentials: 'include',
      });
      if (!res.ok) throw new Error('Failed to delete option');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['options'] });
    },
  });
}

async function fetchOption(id: number): Promise<OptionTrade> {
  const supabase = createClient();
  const { data: { session } } = await supabase.auth.getSession();
  const token = session?.access_token;

  if (!token) {
    throw new Error('User not authenticated');
  }

  const res = await fetch(getFullUrl(apiConfig.endpoints.options.byId(id)), {
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
      throw new Error('Option not found');
    }
    throw new Error('Failed to fetch option');
  }

  const json = await res.json();
  return (json.data ?? json) as OptionTrade;
}

export function useOption(id: number, enabled: boolean = true) {
  return useQuery<OptionTrade>({
    queryKey: ['options', id],
    queryFn: () => fetchOption(id),
    enabled: enabled && id > 0,
    staleTime: 5 * 60 * 1000,
    gcTime: 10 * 60 * 1000,
  });
}


