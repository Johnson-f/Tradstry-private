export type WebSocketConnectionState = 'connecting' | 'connected' | 'disconnected' | 'error';

export type WebSocketEventType =
  | 'stock:created'
  | 'stock:updated'
  | 'stock:deleted'
  | 'option:created'
  | 'option:updated'
  | 'option:deleted'
  | 'note:created'
  | 'note:updated'
  | 'note:deleted'
  | 'playbook:created'
  | 'playbook:updated'
  | 'playbook:deleted'
  | 'trade:tagged'
  | 'trade:untagged'
  | 'chat:message:created'
  | 'chat:message:updated'
  | 'chat:session:created'
  | 'chat:session:updated'
  | 'chat:session:deleted'
  | 'market:quote'
  | 'market:update'
  | 'system:connected'
  | 'system:disconnected'
  | 'system:error'
  | 'ai:analysis:progress'
  | 'ai:analysis:section'
  | 'ai:analysis:complete'
  | 'ai:report:progress'
  | 'ai:report:chunk'
  | 'ai:report:complete'
  | 'notebook_ai_progress'
  | 'notebook_ai_chunk'
  | 'notebook_ai_complete'
  // Normalized versions (underscores -> colons)
  | 'notebook:ai:progress'
  | 'notebook:ai:chunk'
  | 'notebook:ai:complete'
  // Collaboration events
  | 'collaborator:joined'
  | 'collaborator:left'
  | 'collaborator:cursor:move'
  | 'note:content:update'
  | 'note:invitation:received'
  | 'note:invitation:accepted'
  | 'note:invitation:declined'
  | 'note:visibility:changed';

export interface WebSocketEnvelope<TData = unknown> {
  event: WebSocketEventType;
  data: TData;
  timestamp: string;
}

export type WebSocketHandler<TData = unknown> = (data: TData) => void;


