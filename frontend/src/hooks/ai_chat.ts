"use client";

import { useAuth } from "@clerk/nextjs";
import { useMutation } from "@tanstack/react-query";
import * as aiChatService from "@/lib/service/ai_chat";
import type {
  AiChatMessageInput,
  AiChatStreamResult,
  AiChatStreamHandlers,
} from "@/lib/types/ai_chat";

export type AiChatMutationVariables = {
  input: AiChatMessageInput;
  handlers?: AiChatStreamHandlers;
  signal?: AbortSignal;
};

export function useAiChat() {
  const { getToken, isLoaded, isSignedIn } = useAuth();
  const mutation = useMutation<AiChatStreamResult, Error, AiChatMutationVariables>({
    mutationFn: async ({ input, handlers = {}, signal }) => {
      if (!isLoaded || !isSignedIn) {
        throw new Error("You must be signed in to use AI chat");
      }

      return aiChatService.streamAiChat(
        () => getToken(),
        input,
        handlers,
        signal,
      );
    },
  });

  const streamAiChat = async (
    input: AiChatMessageInput,
    handlers: AiChatStreamHandlers = {},
    signal?: AbortSignal,
  ): Promise<AiChatStreamResult> =>
    mutation.mutateAsync({
      input,
      handlers,
      signal,
    });

  return {
    streamAiChat,
    isStreaming: mutation.isPending,
    ...mutation,
    error: mutation.error?.message ?? null,
  };
}
