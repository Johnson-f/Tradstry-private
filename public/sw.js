self.addEventListener('push', (event) => {
  let payload = {};
  try {
    if (event.data) {
      payload = event.data.json();
    }
  } catch (err) {
    console.error('[sw] failed to parse push payload', err);
  }

  const title =
    payload.title ||
    (payload.t === 'price'
      ? `Price alert: ${payload.sid || 'symbol'}`
      : payload.t === 'system'
        ? 'System alert'
        : 'Tradstry');

  const body =
    payload.body ||
    (payload.t === 'price'
      ? `${payload.sid || ''} ${payload.dir || ''} ${payload.th || ''} (now ${payload.p || ''})`
      : payload.message ||
        payload.msg ||
        '');

  const options = {
    body,
    // Prefer provided icon; fall back to absolute URL to avoid mixed-content issues
    icon: payload.icon || `${self.location.origin}/app/icon.png`,
    // Badge is the small app icon shown on the left in many UIs (e.g., macOS/Chrome)
    badge: payload.badge || `${self.location.origin}/app/icon.png`,
    data: {
      url: payload.url || payload.data?.url || '/app/alerts',
      ...payload.data,
      rid: payload.rid,
      eid: payload.eid,
      t: payload.t,
      sid: payload.sid,
    },
    tag: payload.tag || payload.rid || undefined,
  };

  event.waitUntil(self.registration.showNotification(title, options));
});

self.addEventListener('notificationclick', (event) => {
  event.notification.close();
  const targetUrl =
    event.notification?.data?.url ||
    (event.notification?.data?.sid ? `/app/alerts?symbol=${event.notification.data.sid}` : '/');

  event.waitUntil(
    (async () => {
      const allClients = await clients.matchAll({ type: 'window', includeUncontrolled: true });
      const focused = allClients.find((c) => 'focus' in c);

      if (focused) {
        await focused.focus();
        try {
          focused.navigate(targetUrl);
        } catch (_) {
          // ignore navigation errors
        }
      } else {
        await clients.openWindow(targetUrl);
      }
    })()
  );
});

self.addEventListener('pushsubscriptionchange', (event) => {
  event.waitUntil(
    (async () => {
      try {
        const reg = self.registration;
        const current = await reg.pushManager.getSubscription();
        if (!current) {
          // Ask pages to re-register; they hold auth/token + VAPID
          const clientList = await clients.matchAll({ type: 'window', includeUncontrolled: true });
          clientList.forEach((client) =>
            client.postMessage({ type: 'pushsubscriptionchange', reason: event.reason || 'unknown' })
          );
        }
      } catch (err) {
        console.error('[sw] pushsubscriptionchange handling failed', err);
      }
    })()
  );
});
