import { useCallback, useEffect, useMemo, useState } from 'react';
import { apiConfig } from '@/lib/config/api';
import { apiClient } from '@/lib/services/api-client';

const VAPID_PUBLIC_KEY = process.env.NEXT_PUBLIC_VAPID_PUBLIC_KEY as string | undefined;

interface PushSubscriptionKeys {
  p256dh: string;
  auth: string;
}

interface PushSubscriptionJSON {
  endpoint: string;
  keys: PushSubscriptionKeys;
}

const urlBase64ToUint8Array = (base64String: string): Uint8Array => {
  const padding = '='.repeat((4 - (base64String.length % 4)) % 4);
  const base64 = (base64String + padding).replace(/-/g, '+').replace(/_/g, '/');
  const rawData = typeof window !== 'undefined' ? window.atob(base64) : '';
  const outputArray = new Uint8Array(rawData.length);
  for (let i = 0; i < rawData.length; ++i) outputArray[i] = rawData.charCodeAt(i);
  return outputArray;
};

const normalizeSubscription = (sub: PushSubscription): PushSubscriptionJSON => {
  const json = sub.toJSON() as unknown as PushSubscriptionJSON;
  if (!json.keys?.p256dh || !json.keys?.auth || !json.endpoint) {
    throw new Error('Invalid push subscription keys');
  }
  return json;
};

export function useWebPush() {
  const [permission, setPermission] = useState<NotificationPermission>('default');
  const [isSubscribing, setIsSubscribing] = useState(false);
  const isSupported = useMemo(
    () => typeof window !== 'undefined' && 'serviceWorker' in navigator && 'PushManager' in window,
    []
  );

  useEffect(() => {
    if (typeof window !== 'undefined') {
      setPermission(Notification.permission);
    }
  }, []);

  const getRegistration = useCallback(async () => {
    if (!isSupported) return null;
    const existing = await navigator.serviceWorker.getRegistration();
    if (existing) return existing;
    return navigator.serviceWorker.register('/sw.js');
  }, [isSupported]);

  const persistSubscription = useCallback(async (sub: PushSubscriptionJSON) => {
    return apiClient.post(apiConfig.endpoints.alerts.subscribe, {
      endpoint: sub.endpoint,
      keys: { p256dh: sub.keys.p256dh, auth: sub.keys.auth },
      ua: typeof navigator !== 'undefined' ? navigator.userAgent : undefined,
      topics: ['price', 'system'],
    });
  }, []);

  const subscribe = useCallback(async () => {
    if (!isSupported) throw new Error('Web Push not supported');
    if (!VAPID_PUBLIC_KEY) throw new Error('Missing VAPID public key');

    setIsSubscribing(true);
    try {
      const reg = await getRegistration();
      if (!reg) throw new Error('Service worker registration failed');

      // Ensure an active/ready SW before subscribing
      const readyReg =
        (await navigator.serviceWorker.ready.catch(() => reg)) || reg;

      const existing = await reg.pushManager.getSubscription();
      if (existing) {
        return persistSubscription(normalizeSubscription(existing));
      }

      const applicationServerKey = urlBase64ToUint8Array(VAPID_PUBLIC_KEY);
      const sub = await readyReg.pushManager.subscribe({
        userVisibleOnly: true,
        applicationServerKey: applicationServerKey as BufferSource,
      });
      return persistSubscription(normalizeSubscription(sub));
    } finally {
      setIsSubscribing(false);
    }
  }, [getRegistration, isSupported, persistSubscription]);

  // Re-register when SW notifies about subscription change
  useEffect(() => {
    if (!isSupported) return;
    const handler = (event: MessageEvent) => {
      if (event.data?.type === 'pushsubscriptionchange') {
        void subscribe();
      }
    };
    navigator.serviceWorker.addEventListener('message', handler);
    return () => navigator.serviceWorker.removeEventListener('message', handler);
  }, [isSupported, subscribe]);

  const unsubscribe = useCallback(async () => {
    const reg = await navigator.serviceWorker.getRegistration();
    const sub = await reg?.pushManager.getSubscription();
    const endpoint = sub?.endpoint;
    await sub?.unsubscribe();
    if (endpoint) {
      await apiClient.post(apiConfig.endpoints.alerts.unsubscribe, { endpoint });
    }
  }, []);

  const requestPermission = useCallback(async () => {
    if (!isSupported) return 'denied' as NotificationPermission;
    const p = await Notification.requestPermission();
    setPermission(p);
    return p;
  }, [isSupported]);

  const sendTest = useCallback(async () => {
    return apiClient.post(apiConfig.endpoints.alerts.test);
  }, []);

  return {
    isSupported,
    permission,
    isSubscribing,
    requestPermission,
    subscribe,
    unsubscribe,
    registerServiceWorker: getRegistration,
    sendTest,
  };
}
