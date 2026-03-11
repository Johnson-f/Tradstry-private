'use client';

import { useEffect, useState } from 'react';
import { Wifi, WifiOff, Loader2, AlertCircle, RefreshCw } from 'lucide-react';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import type { ConnectionStatus } from '@/components/notebook/rich_editor/plugins';

interface ConnectionStatusIndicatorProps {
  status: ConnectionStatus;
  error?: string | null;
  onRetry?: () => void;
  className?: string;
}

const statusConfig: Record<ConnectionStatus, {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  description: string;
  color: string;
  animate?: boolean;
}> = {
  connecting: {
    icon: Loader2,
    label: 'Connecting',
    description: 'Establishing connection to collaboration server...',
    color: 'text-yellow-500',
    animate: true,
  },
  connected: {
    icon: Wifi,
    label: 'Connected',
    description: 'Real-time collaboration is active',
    color: 'text-green-500',
  },
  disconnected: {
    icon: WifiOff,
    label: 'Disconnected',
    description: 'Connection lost. Your changes are saved locally.',
    color: 'text-gray-400',
  },
  reconnecting: {
    icon: RefreshCw,
    label: 'Reconnecting',
    description: 'Attempting to reconnect...',
    color: 'text-yellow-500',
    animate: true,
  },
  error: {
    icon: AlertCircle,
    label: 'Connection Error',
    description: 'Unable to connect. Please try again.',
    color: 'text-red-500',
  },
};

export function ConnectionStatusIndicator({
  status,
  error,
  onRetry,
  className,
}: ConnectionStatusIndicatorProps) {
  const [showToast, setShowToast] = useState(false);
  const config = statusConfig[status];
  const Icon = config.icon;

  // Show toast notification on status change (except for connected)
  useEffect(() => {
    if (status === 'error' || status === 'disconnected') {
      setShowToast(true);
      const timer = setTimeout(() => setShowToast(false), 5000);
      return () => clearTimeout(timer);
    }
  }, [status]);

  return (
    <TooltipProvider>
      <div className={cn('flex items-center gap-2', className)}>
        <Tooltip>
          <TooltipTrigger asChild>
            <div className="flex items-center gap-1.5 cursor-default">
              <Icon
                className={cn(
                  'h-4 w-4',
                  config.color,
                  config.animate && 'animate-spin'
                )}
              />
              <span className={cn('text-xs font-medium', config.color)}>
                {config.label}
              </span>
            </div>
          </TooltipTrigger>
          <TooltipContent side="bottom" className="max-w-xs">
            <div className="space-y-1">
              <p className="text-sm">{config.description}</p>
              {error && (
                <p className="text-xs text-red-400">{error}</p>
              )}
              {status === 'error' && onRetry && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={onRetry}
                  className="mt-2 w-full"
                >
                  <RefreshCw className="h-3 w-3 mr-1" />
                  Retry Connection
                </Button>
              )}
            </div>
          </TooltipContent>
        </Tooltip>
      </div>
    </TooltipProvider>
  );
}

// Compact version for toolbar
interface ConnectionStatusDotProps {
  status: ConnectionStatus;
  className?: string;
}

export function ConnectionStatusDot({ status, className }: ConnectionStatusDotProps) {
  const colorMap: Record<ConnectionStatus, string> = {
    connecting: 'bg-yellow-500 animate-pulse',
    connected: 'bg-green-500',
    disconnected: 'bg-gray-400',
    reconnecting: 'bg-yellow-500 animate-pulse',
    error: 'bg-red-500',
  };

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <span
            className={cn(
              'inline-block h-2 w-2 rounded-full',
              colorMap[status],
              className
            )}
          />
        </TooltipTrigger>
        <TooltipContent side="bottom">
          <span className="text-xs">{statusConfig[status].label}</span>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

export default ConnectionStatusIndicator;
