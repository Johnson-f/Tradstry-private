'use client';

import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import type { CollaboratorPresence } from '@/lib/types/collaboration';

interface CollaboratorAvatarsProps {
  participants: CollaboratorPresence[];
  maxVisible?: number;
  size?: 'sm' | 'md' | 'lg';
}

const sizeClasses = {
  sm: 'h-6 w-6 text-[10px]',
  md: 'h-8 w-8 text-xs',
  lg: 'h-10 w-10 text-sm',
};

export function CollaboratorAvatars({
  participants,
  maxVisible = 4,
  size = 'md',
}: CollaboratorAvatarsProps) {
  const visibleParticipants = participants.slice(0, maxVisible);
  const remainingCount = participants.length - maxVisible;

  const getInitials = (email: string, name?: string) => {
    if (name) return name.slice(0, 2).toUpperCase();
    return email.slice(0, 2).toUpperCase();
  };

  const getStatusColor = (isActive: boolean) => {
    return isActive ? 'bg-green-500' : 'bg-gray-400';
  };

  if (participants.length === 0) return null;

  return (
    <TooltipProvider>
      <div className="flex items-center -space-x-2">
        {visibleParticipants.map((participant) => (
          <Tooltip key={participant.user_id}>
            <TooltipTrigger asChild>
              <div className="relative">
                <Avatar
                  className={`${sizeClasses[size]} border-2 border-background ring-2`}
                  style={{
                    ringColor: participant.cursor?.user_color || '#4ECDC4',
                  }}
                >
                  <AvatarFallback
                    style={{
                      backgroundColor: participant.cursor?.user_color || '#4ECDC4',
                      color: 'white',
                    }}
                  >
                    {getInitials(participant.user_email, participant.user_name)}
                  </AvatarFallback>
                </Avatar>
                <span
                  className={`absolute bottom-0 right-0 h-2 w-2 rounded-full border border-background ${getStatusColor(
                    participant.is_active
                  )}`}
                />
              </div>
            </TooltipTrigger>
            <TooltipContent>
              <div className="text-sm">
                <div className="font-medium">
                  {participant.user_name || participant.user_email}
                </div>
                <div className="text-xs text-muted-foreground capitalize">
                  {participant.role} • {participant.is_active ? 'Active' : 'Away'}
                </div>
              </div>
            </TooltipContent>
          </Tooltip>
        ))}

        {remainingCount > 0 && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Avatar className={`${sizeClasses[size]} border-2 border-background bg-muted`}>
                <AvatarFallback className="text-muted-foreground">
                  +{remainingCount}
                </AvatarFallback>
              </Avatar>
            </TooltipTrigger>
            <TooltipContent>
              <div className="text-sm">
                {participants.slice(maxVisible).map((p) => (
                  <div key={p.user_id}>{p.user_name || p.user_email}</div>
                ))}
              </div>
            </TooltipContent>
          </Tooltip>
        )}
      </div>
    </TooltipProvider>
  );
}
