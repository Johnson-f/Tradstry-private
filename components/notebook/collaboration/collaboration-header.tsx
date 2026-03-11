'use client';

import { ShareButton } from './share-button';
import { CollaboratorAvatars } from './collaborator-avatars';
import { ConnectionStatusDot } from './connection-status';
import type { CollaboratorPresence } from '@/lib/types/collaboration';
import type { ConnectionStatus } from '@/components/notebook/rich_editor/plugins';

interface CollaborationHeaderProps {
  noteId?: string;
  noteTitle: string;
  connectionStatus?: ConnectionStatus;
  participants?: CollaboratorPresence[];
  showConnectionStatus?: boolean;
}

export function CollaborationHeader({
  noteId,
  noteTitle,
  connectionStatus = 'disconnected',
  participants = [],
  showConnectionStatus = false,
}: CollaborationHeaderProps) {
  // Only show collaboration UI if we have a note ID
  const hasCollaboration = noteId && (participants.length > 0 || showConnectionStatus);

  return (
    <div className="flex items-center gap-3">
      {/* Connection status indicator */}
      {showConnectionStatus && noteId && (
        <ConnectionStatusDot status={connectionStatus} />
      )}

      {/* Collaborator avatars */}
      {participants.length > 0 && (
        <CollaboratorAvatars
          participants={participants}
          maxVisible={3}
          size="sm"
        />
      )}

      {/* Share button */}
      <ShareButton noteId={noteId} noteTitle={noteTitle} />
    </div>
  );
}

export default CollaborationHeader;
