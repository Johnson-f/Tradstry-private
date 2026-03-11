'use client';
// This file contains all the hook for both websocket & normal websocket 
import { useEffect } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useWs } from '@/lib/websocket/provider';
import playbookService from '@/lib/services/playbook-service';
import type { Playbook, CreatePlaybookRequest, UpdatePlaybookRequest, PlaybookAnalytics, PlaybookRule, CreateRuleRequest } from '@/lib/types/playbook';

async function fetchPlaybooks(): Promise<Playbook[]> {
  const res = await playbookService.listPlaybooks();
  if (!res.success) throw new Error(res.message || 'Failed to fetch playbooks');
  return (res.data ?? []) as Playbook[];
}

export function usePlaybooks() {
  const queryClient = useQueryClient();
  const { subscribe } = useWs();

  const query = useQuery<Playbook[]>({
    queryKey: ['playbooks'],
    queryFn: fetchPlaybooks,
    staleTime: 5 * 60 * 1000,
  });

  useEffect(() => {
    // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
    const unsubCreate = subscribe('playbook:created', (playbook: Playbook) => {
      queryClient.setQueryData<Playbook[]>(['playbooks'], (prev) => prev ? [playbook, ...prev] : [playbook]);
    });
    // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
    const unsubUpdate = subscribe('playbook:updated', (playbook: Playbook) => {
      queryClient.setQueryData<Playbook[]>(['playbooks'], (prev) => prev?.map(p => p.id === playbook.id ? playbook : p) ?? [playbook]);
    });
    // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
    const unsubDelete = subscribe('playbook:deleted', (payload: { id: string }) => {
      queryClient.setQueryData<Playbook[]>(['playbooks'], (prev) => prev?.filter(p => p.id !== payload.id) ?? []);
    });
    return () => { unsubCreate(); unsubUpdate(); unsubDelete(); };
  }, [subscribe, queryClient]);

  // CRUD helpers via service (REST). WS will fan-out updates to all tabs/clients.
  const createPlaybook = async (payload: CreatePlaybookRequest) => {
    const res = await playbookService.createPlaybook(payload);
    if (!res.success) throw new Error(res.message || 'Failed to create playbook');
    // Optimistically add to cache; WS will also deliver authoritative event
    const created = res.data as Playbook;
    queryClient.setQueryData<Playbook[]>(['playbooks'], (prev) => prev ? [created, ...prev] : [created]);
    return created;
  };

  const updatePlaybook = async (id: string, payload: UpdatePlaybookRequest) => {
    const res = await playbookService.updatePlaybook(id, payload);
    if (!res.success) throw new Error(res.message || 'Failed to update playbook');
    const updated = res.data as Playbook;
    queryClient.setQueryData<Playbook[]>(['playbooks'], (prev) => prev?.map(p => p.id === updated.id ? updated : p) ?? [updated]);
    return updated;
  };

  const deletePlaybook = async (id: string) => {
    const res = await playbookService.deletePlaybook(id);
    if (!res.success) throw new Error(res.message || 'Failed to delete playbook');
    queryClient.setQueryData<Playbook[]>(['playbooks'], (prev) => prev?.filter(p => p.id !== id) ?? []);
    return true;
  };

  return { ...query, createPlaybook, updatePlaybook, deletePlaybook };
}

// Unique REST (non-WebSocket) hooks
export function usePlaybookAnalytics(playbookId: string, timeRange?: string) {
  const { subscribe } = useWs();
  const query = useQuery<PlaybookAnalytics | null>({
    queryKey: ['playbook', playbookId, 'analytics', timeRange],
    queryFn: async () => {
      const res = await playbookService.getPlaybookAnalytics(playbookId, timeRange);
      if (!res.success) throw new Error(res.message || 'Failed to fetch analytics');
      return (res.data ?? null) as PlaybookAnalytics | null;
    },
    enabled: !!playbookId,
    staleTime: 5 * 60 * 1000,
  });

  // Refetch analytics when the specific playbook changes
  useEffect(() => {
    // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
    const unsubUpdate = subscribe('playbook:updated', (p: { id: string }) => {
      if (p?.id === playbookId) query.refetch();
    });
    // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
    const unsubDelete = subscribe('playbook:deleted', (p: { id: string }) => {
      if (p?.id === playbookId) query.refetch();
    });
    return () => { unsubUpdate(); unsubDelete(); };
  }, [subscribe, playbookId, query]);

  return query;
}

export function useAllPlaybooksAnalytics(timeRange?: string) {
  const { subscribe } = useWs();
  const query = useQuery<PlaybookAnalytics[]>({
    queryKey: ['playbooks', 'analytics', timeRange],
    queryFn: async () => {
      const res = await playbookService.getAllPlaybooksAnalytics(timeRange);
      if (!res.success) throw new Error(res.message || 'Failed to fetch analytics');
      return (res.data ?? []) as PlaybookAnalytics[];
    },
    staleTime: 5 * 60 * 1000,
  });

  // Refetch when any playbook is created/updated/deleted
  useEffect(() => {
    const unsubCreate = subscribe('playbook:created', () => query.refetch());
    const unsubUpdate = subscribe('playbook:updated', () => query.refetch());
    const unsubDelete = subscribe('playbook:deleted', () => query.refetch());
    return () => { unsubCreate(); unsubUpdate(); unsubDelete(); };
  }, [subscribe, query]);

  return query;
}

export function usePlaybookRules(playbookId: string) {
  const { subscribe } = useWs();
  const queryClient = useQueryClient();

  const query = useQuery<PlaybookRule[]>({
    queryKey: ['playbook', playbookId, 'rules'],
    queryFn: async () => {
      const res = await playbookService.getRules(playbookId);
      if (!res.success) throw new Error(res.message || 'Failed to fetch rules');
      return (res.data ?? []) as PlaybookRule[];
    },
    enabled: !!playbookId,
    staleTime: 5 * 60 * 1000,
  });

  useEffect(() => {
    // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
    const unsubUpdate = subscribe('playbook:updated', (p: { id: string }) => {
      if (p?.id === playbookId) query.refetch();
    });
    return () => { unsubUpdate(); };
  }, [subscribe, playbookId, query]);

  const createRule = async (payload: CreateRuleRequest) => {
    const res = await playbookService.createRule(playbookId, payload);
    if (!res.success) throw new Error(res.message || 'Failed to create rule');
    queryClient.invalidateQueries({ queryKey: ['playbook', playbookId, 'rules'] });
    return res.data as PlaybookRule;
  };

  const deleteRule = async (ruleId: string) => {
    const res = await playbookService.deleteRule(playbookId, ruleId);
    if (!res.success) throw new Error(res.message || 'Failed to delete rule');
    queryClient.invalidateQueries({ queryKey: ['playbook', playbookId, 'rules'] });
    return true;
  };

  return { ...query, createRule, deleteRule };
}


