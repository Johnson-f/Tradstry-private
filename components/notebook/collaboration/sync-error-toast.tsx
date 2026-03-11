'use client';

import { useEffect } from 'react';
import { toast } from 'sonner';
import { AlertCircle, RefreshCw, Wifi, WifiOff } from 'lucide-react';
import type { ConnectionStatus } from '@/components/notebook/rich_editor/plugins';

interface SyncErrorToastProps {
  status: ConnectionStatus;
  error?: string | null;
  onRetry?: () => void;
}

// Track last shown status to avoid duplicate toasts
let lastShownStatus: ConnectionStatus | null = null;
let lastShownError: string | null = null;

export function useSyncErrorToast({
  status,
  error,
  onRetry,
}: SyncErrorToastProps) {
  useEffect(() => {
    // Avoid showing duplicate toasts
    if (status === lastShownStatus && error === lastShownError) {
      return;
    }

    // Show toast based on status
    switch (status) {
      case 'connected':
        // Only show reconnected toast if we were previously disconnected
        if (lastShownStatus === 'disconnected' || lastShownStatus === 'reconnecting' || lastShownStatus === 'error') {
          toast.success('Connected', {
            description: 'Real-time collaboration is active',
            icon: <Wifi className="h-4 w-4 text-green-500" />,
            duration: 3000,
          });
        }
        break;

      case 'disconnected':
        toast.warning('Disconnected', {
          description: 'Your changes are saved locally. Reconnecting...',
          icon: <WifiOff className="h-4 w-4 text-yellow-500" />,
          duration: 5000,
        });
        break;

      case 'reconnecting':
        // Don't show toast for reconnecting - it's handled by disconnected
        break;

      case 'error':
        toast.error('Connection Error', {
          description: error || 'Unable to connect to collaboration server',
          icon: <AlertCircle className="h-4 w-4 text-red-500" />,
          duration: 10000,
          action: onRetry ? {
            label: 'Retry',
            onClick: onRetry,
          } : undefined,
        });
        break;
    }

    lastShownStatus = status;
    lastShownError = error || null;
  }, [status, error, onRetry]);
}

// Reset toast tracking (useful for testing or when component unmounts)
export function resetSyncErrorToastTracking() {
  lastShownStatus = null;
  lastShownError = null;
}

export default useSyncErrorToast;
