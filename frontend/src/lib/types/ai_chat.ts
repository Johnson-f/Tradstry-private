export interface AiChatMessageInput {
  message: string;
  sessionId?: string | null;
}

export interface AiChatDeltaEvent {
  event: "delta";
  requestId: string;
  sessionId: string;
  text: string;
}

export interface AiChatCompletedEvent {
  event: "completed";
  requestId: string;
  sessionId: string;
  text: string;
  promotedMemoryUris: string[];
}

export interface AiChatErrorEvent {
  event: "error";
  requestId: string;
  sessionId: string;
  message: string;
}

export type AiChatStreamEvent =
  | AiChatDeltaEvent
  | AiChatCompletedEvent
  | AiChatErrorEvent;

export interface AiChatStreamResult {
  requestId: string;
  sessionId: string;
  text: string;
  promotedMemoryUris: string[];
}

export interface AiChatStreamHandlers {
  onDelta?: (chunk: string) => void;
  onCompleted?: (event: AiChatCompletedEvent) => void;
  onError?: (message: string) => void;
}
