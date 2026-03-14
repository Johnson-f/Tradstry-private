"use client";

import { FormEvent, useRef, useState } from "react";

import { DashboardRoutePage } from "@/components/dashboard-route-page";
import { useAiChat } from "@/hooks/ai_chat";

type ChatRole = "user" | "assistant" | "error";

type ChatMessage = {
  id: number;
  role: ChatRole;
  text: string;
};

export default function AIChatPage() {
  const { streamAiChat, isStreaming, error: mutationError } = useAiChat();
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const controllerRef = useRef<AbortController | null>(null);
  const messageId = useRef(0);

  const addMessage = (message: Omit<ChatMessage, "id">) =>
    setMessages((prev) => [...prev, { ...message, id: ++messageId.current }]);

  const setLastAssistantText = (updater: (text: string) => string) => {
    setMessages((prev) => {
      const next = [...prev];
      if (next.length === 0) {
        return prev;
      }

      for (let i = next.length - 1; i >= 0; i -= 1) {
        if (next[i].role === "assistant") {
          next[i] = { ...next[i], text: updater(next[i].text) };
          return next;
        }
      }
      return prev;
    });
  };

  const appendAssistantChunk = (chunk: string) => {
    setLastAssistantText((text) => text + chunk);
  };

  const appendError = (message: string) => {
    addMessage({ role: "error", text: message });
  };

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();

    const trimmedInput = input.trim();
    if (!trimmedInput || isSubmitting || isStreaming) {
      return;
    }

    const controller = new AbortController();
    controllerRef.current = controller;

    setInput("");
    setIsSubmitting(true);

    addMessage({ role: "user", text: trimmedInput });
    addMessage({ role: "assistant", text: "" });

    try {
      const result = await streamAiChat(
        {
          message: trimmedInput,
          sessionId,
        },
        {
          onDelta: appendAssistantChunk,
          onCompleted: (event) => {
            setSessionId(event.sessionId);
            setLastAssistantText(() => event.text);
          },
          onError: (message) => {
            appendError(`AI stream error: ${message}`);
          },
        },
        controller.signal,
      );

      setSessionId(result.sessionId);
      setLastAssistantText(() => result.text);
    } catch (err) {
      const message =
        err instanceof Error ? err.message : "Something went wrong while streaming.";
      if (!messages.some((msg) => msg.role === "error" && msg.text === `AI stream error: ${message}`)) {
        appendError(`AI stream error: ${message}`);
      }
    } finally {
      setIsSubmitting(false);
      controllerRef.current = null;
    }
  };

  const handleStop = () => {
    controllerRef.current?.abort();
    appendError("AI stream cancelled");
    setIsSubmitting(false);
  };

  return (
    <>
      <DashboardRoutePage title="AI Chat" />
      <section className="mt-4 rounded-xl border bg-background p-6">
        <h2 className="text-lg font-medium">AI chat</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Start a conversation with your trading assistant. Sessions are kept in-memory and
          threaded by the current chat session id.
        </p>

        <div className="mt-4 max-h-[26rem] overflow-y-auto rounded-md border bg-muted/40 p-4">
          {messages.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              Send a message to start.
            </p>
          ) : (
            <ul className="space-y-3">
              {messages.map((message) => (
                <li
                  key={message.id}
                  className={`rounded-md p-3 ${
                    message.role === "user"
                      ? "bg-primary/10 text-right"
                      : message.role === "error"
                        ? "bg-destructive/10 text-destructive"
                        : "bg-muted text-left"
                  }`}
                >
                  <p className="text-xs font-semibold uppercase tracking-wide">
                    {message.role}
                  </p>
                  <p className="mt-1 whitespace-pre-wrap text-sm">{message.text || "…"}</p>
                </li>
              ))}
            </ul>
          )}
        </div>

        <form className="mt-4 flex flex-col gap-2" onSubmit={handleSubmit}>
          <textarea
            className="min-h-[110px] rounded-md border bg-background p-3 text-sm"
            placeholder="Ask your trading assistant..."
            value={input}
            onChange={(event) => setInput(event.target.value)}
            disabled={isSubmitting || isStreaming}
          />
          <div className="flex items-center gap-2">
            <button
              type="submit"
              className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground disabled:opacity-50"
              disabled={isSubmitting || isStreaming || !input.trim()}
            >
              {isSubmitting || isStreaming ? "Streaming..." : "Send"}
            </button>
            {(isSubmitting || isStreaming) && (
              <button
                type="button"
                onClick={handleStop}
                className="rounded-md border px-4 py-2 text-sm"
              >
                Cancel
              </button>
            )}
          </div>
        </form>

        {mutationError && (
          <p className="mt-2 text-sm text-destructive">Request error: {mutationError}</p>
        )}
      </section>
    </>
  );
}
