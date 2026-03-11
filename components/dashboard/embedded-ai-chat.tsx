// components/dashboard/embedded-ai-chat.tsx
// Updated version with floating input (no background container)

"use client";

import { useState, useEffect, useRef, useCallback } from "react";
import { useRouter } from "next/navigation";
import { useAIChat } from "@/hooks/use-ai-chat";

import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { Loader2, Plus, ArrowUp } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupText,
  InputGroupTextarea,
} from "@/components/ui/input-group";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Separator } from "@/components/ui/separator";

import { ChatMessage, StreamingMessage, AIThinkingIndicator } from "@/components/chat/chat-message";

interface QuickAction {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  prompt: string;
}

interface EmbeddedAIChatProps {
  className?: string;
  defaultExpanded?: boolean;
}

const QUICK_ACTIONS: QuickAction[] = [];
const MAX_MESSAGE_LENGTH = 4000;

const QuickActionButton = ({
  action,
  onAction,
}: {
  action: QuickAction;
  onAction: (prompt: string) => void;
}) => (
  <Button
    variant="outline"
    onClick={() => onAction(action.prompt)}
    className="border-gray-700 bg-gray-800 text-gray-300 hover:bg-gray-700 hover:text-white rounded-full px-4 py-2 text-sm transition-colors"
  >
    <action.icon className="h-4 w-4 mr-2" />
    {action.label}
  </Button>
);

const ErrorDisplay = ({
  error,
  onRetry,
}: {
  error: { message: string };
  onRetry: () => void;
}) => (
  <div className="mt-4 p-3 bg-red-900/50 border border-red-700 rounded-lg text-red-300 text-sm">
    <div className="flex items-center justify-center gap-2">
      <span>Error: {error.message}</span>
      <Button
        variant="ghost"
        size="sm"
        onClick={onRetry}
        className="ml-2 text-red-300 hover:text-white"
      >
        Retry
      </Button>
    </div>
  </div>
);

export default function EmbeddedAIChat({
  className,
}: EmbeddedAIChatProps) {
  const [message, setMessage] = useState("");
  const [currentSessionId, setCurrentSessionId] = useState<string | null>(null);
  const [isMinimizing, setIsMinimizing] = useState(false);
  const [userScrolled, setUserScrolled] = useState(false);
  const [chatMode, setChatMode] = useState<"auto" | "agent" | "manual">("auto");

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const messagesContainerRef = useRef<HTMLDivElement>(null);
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  const autoScrollTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  const router = useRouter();
  const {
    messages,
    isLoading,
    isStreaming,
    error,
    createSession,
    loadSession,
    clearError,
  } = useAIChat();

  const streamingContent = isStreaming && messages.length > 0 
    ? messages[messages.length - 1]?.content 
    : "";

  useEffect(() => {
    if (currentSessionId) {
      loadSession(currentSessionId);
    }
  }, [currentSessionId, loadSession]);

  const scrollToBottom = useCallback((behavior: ScrollBehavior = "smooth", force: boolean = false) => {
    if (!force && userScrolled && !isStreaming) {
      return;
    }

    if (messagesEndRef.current) {
      if (autoScrollTimeoutRef.current) {
        clearTimeout(autoScrollTimeoutRef.current);
      }

      requestAnimationFrame(() => {
        if (messagesEndRef.current) {
          messagesEndRef.current.scrollIntoView({ 
            behavior, 
            block: "end",
            inline: "nearest" 
          });
        }
      });
    }
  }, [userScrolled, isStreaming]);

  const handleScroll = useCallback(() => {
    const scrollArea = scrollAreaRef.current;
    if (!scrollArea) return;

    const { scrollTop, scrollHeight, clientHeight } = scrollArea;
    const isAtBottom = Math.abs(scrollHeight - clientHeight - scrollTop) < 50;
    
    if (!isAtBottom && !isStreaming) {
      setUserScrolled(true);
    } else if (isAtBottom) {
      setUserScrolled(false);
    }
  }, [isStreaming]);

  useEffect(() => {
    if (isStreaming) {
      if (autoScrollTimeoutRef.current) {
        clearTimeout(autoScrollTimeoutRef.current);
      }

      autoScrollTimeoutRef.current = setTimeout(() => {
        scrollToBottom("smooth", true);
      }, 16);
    }

    return () => {
      if (autoScrollTimeoutRef.current) {
        clearTimeout(autoScrollTimeoutRef.current);
        autoScrollTimeoutRef.current = null;
      }
    };
  }, [isStreaming, scrollToBottom, streamingContent]);

  useEffect(() => {
    if (isStreaming) {
      setUserScrolled(false);
      scrollToBottom("smooth", true);
    }
  }, [isStreaming, scrollToBottom]);

  useEffect(() => {
    if (!isStreaming && !userScrolled) {
      scrollToBottom("auto");
    }
  }, [messages, isLoading, isStreaming, userScrolled, scrollToBottom]);

  useEffect(() => {
    const scrollArea = scrollAreaRef.current;
    if (!scrollArea || !isStreaming) return;

    const observer = new ResizeObserver(() => {
      if (isStreaming && !userScrolled) {
        scrollToBottom("smooth", true);
      }
    });

    if (messagesContainerRef.current) {
      observer.observe(messagesContainerRef.current);
    }

    return () => {
      observer.disconnect();
    };
  }, [isStreaming, userScrolled, scrollToBottom]);

  useEffect(() => {
    if (inputRef.current && !isLoading) {
      inputRef.current.focus();
    }
  }, [isLoading]);

  useEffect(() => {
    return () => {
      if (autoScrollTimeoutRef.current) {
        clearTimeout(autoScrollTimeoutRef.current);
      }
    };
  }, []);

  const handleSendMessage = useCallback(
    async (messageText?: string) => {
      const textToSend = messageText || message.trim();

      if (!textToSend || textToSend.length > MAX_MESSAGE_LENGTH) {
        return;
      }

      try {
        setUserScrolled(false);
        setMessage("");
        
        if (!currentSessionId) {
          setIsMinimizing(true);
        }

        if (currentSessionId) {
          router.push(`/app/chat/${currentSessionId}`);
          return;
        }

        // Create session first
        const sessionId = await createSession("New Chat");
        
        // Store message and mode in localStorage for hydration
        try {
          const storageKey = `ai_chat:${sessionId}:messages`;
          const modeKey = `ai_chat:${sessionId}:mode`;
          const pendingMessageKey = `ai_chat:${sessionId}:pending`;
          
          const existingRaw = typeof window !== "undefined" ? window.localStorage.getItem(storageKey) : null;
          const existingMessages: unknown = existingRaw ? JSON.parse(existingRaw) : [];

          const isArray = Array.isArray(existingMessages);
          const messagesArray = isArray ? (existingMessages as unknown[]) : [];

          const generatedId =
            typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
              ? crypto.randomUUID()
              : `local-${Date.now()}`;

          const localMessage = {
            id: generatedId,
            session_id: sessionId,
            content: textToSend,
            message_type: "user_question",
            role: "user",
            created_at: new Date().toISOString(),
          };

          messagesArray.push(localMessage);
          if (typeof window !== "undefined") {
            window.localStorage.setItem(storageKey, JSON.stringify(messagesArray));
            window.localStorage.setItem(modeKey, chatMode);
            // Store the pending message to be sent after redirect
            window.localStorage.setItem(pendingMessageKey, textToSend);
          }
        } catch (e) {
          console.error("Failed to persist message locally", e);
        }
        
        setCurrentSessionId(sessionId);
        router.push(`/app/chat/${sessionId}`);
        
      } catch (error) {
        console.error("Failed to send message:", error);
        setIsMinimizing(false);
      }
    },
    [message, currentSessionId, createSession, router, chatMode],
  );

  const handleQuickAction = useCallback(
    (prompt: string) => {
      handleSendMessage(prompt);
    },
    [handleSendMessage],
  );

  const handleKeyPress = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSendMessage();
      }
    },
    [handleSendMessage],
  );

  const handleNewChat = useCallback(() => {
    setCurrentSessionId(null);
    setMessage("");
    setUserScrolled(false);
    if (inputRef.current) {
      inputRef.current.focus();
    }
  }, []);

  const handleInputChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      const value = e.target.value;
      if (value.length <= MAX_MESSAGE_LENGTH) {
        setMessage(value);
      }
    },
    [],
  );

  const canSendMessage =
    !isLoading && !isStreaming && !isMinimizing && message.trim().length > 0;
  const hasMessages = messages.length > 0;

  const displayMessages = isStreaming && messages.length > 0
    ? messages.slice(0, -1)
    : messages;

  return (
    <div
      className={cn(
        "w-full max-w-4xl mx-auto transition-all duration-700 ease-in-out",
        isMinimizing && "transform translate-y-full opacity-0 scale-95",
        className,
      )}
    >
      {(hasMessages || isLoading || isStreaming) && (
        <section aria-label="Chat messages" className="mb-6">
          <ScrollArea 
            className="h-[650px]" 
            ref={scrollAreaRef}
            onScrollCapture={handleScroll}
          >
            <div className="space-y-4 py-4" ref={messagesContainerRef}>
                {displayMessages.map((msg) => (
                  <ChatMessage key={msg.id} message={msg} />
                ))}
                
                {isLoading && !isStreaming && <AIThinkingIndicator />}
                
                {isStreaming && streamingContent && (
                  <StreamingMessage content={streamingContent} />
                )}
                
                <div ref={messagesEndRef} className="h-1" data-messages-end />
              </div>
            </ScrollArea>
            
            {userScrolled && isStreaming && (
              <div className="flex justify-center mt-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    setUserScrolled(false);
                    scrollToBottom("smooth", true);
                  }}
                  className="bg-orange-500/20 border-orange-500/30 text-orange-300 hover:bg-orange-500/30 text-xs"
                >
                  New messages ↓
                </Button>
              </div>
            )}
          </section>
        )}

        {/* Floating Input Section - No Background */}
        <section className="relative" aria-label="Message input">
          <InputGroup className="bg-gray-800 border-gray-700 focus-within:border-gray-600 shadow-xl">
            <InputGroupTextarea
              ref={inputRef}
              placeholder="Ask, Search or Chat..."
              value={message}
              onChange={handleInputChange}
              onKeyDown={handleKeyPress}
              disabled={isLoading || isStreaming || isMinimizing}
              maxLength={MAX_MESSAGE_LENGTH}
              className="text-white placeholder:text-gray-400"
              aria-label="Type your message"
            />

            <InputGroupAddon align="block-end">
              <InputGroupButton
                variant="outline"
                className="rounded-full"
                size="icon-xs"
                onClick={handleNewChat}
                disabled={isLoading || isStreaming || isMinimizing}
                aria-label="Start new chat"
              >
                <Plus />
              </InputGroupButton>

              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <InputGroupButton variant="ghost">
                    {chatMode === "auto" && "Auto"}
                    {chatMode === "agent" && "Agent"}
                    {chatMode === "manual" && "Manual"}
                  </InputGroupButton>
                </DropdownMenuTrigger>
                <DropdownMenuContent
                  side="top"
                  align="start"
                  className="[--radius:0.95rem]"
                >
                  <DropdownMenuItem onClick={() => setChatMode("auto")}>
                    Auto
                    <span className="ml-2 text-xs text-muted-foreground">No tools</span>
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => setChatMode("agent")}>
                    Agent
                    <span className="ml-2 text-xs text-muted-foreground">With tools</span>
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => setChatMode("manual")}>
                    Manual
                    <span className="ml-2 text-xs text-muted-foreground">Custom</span>
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>

              {message.length > MAX_MESSAGE_LENGTH * 0.8 && (
                <InputGroupText className="ml-auto">
                  {message.length}/{MAX_MESSAGE_LENGTH}
                </InputGroupText>
              )}

              <Separator orientation="vertical" className="!h-4" />

              <InputGroupButton
                variant="default"
                className={cn(
                  "rounded-full disabled:bg-gray-600 transition-colors",
                  chatMode === "agent" 
                    ? "bg-orange-500 hover:bg-orange-600" 
                    : "bg-blue-500 hover:bg-blue-600"
                )}
                size="icon-xs"
                onClick={() => handleSendMessage()}
                disabled={!canSendMessage || isMinimizing}
                aria-label="Send message"
                title={chatMode === "agent" ? "Send with tools (fetches live market data)" : "Send without tools"}
              >
                {isLoading || isStreaming || isMinimizing ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <ArrowUp />
                )}
                <span className="sr-only">Send</span>
              </InputGroupButton>
            </InputGroupAddon>
          </InputGroup>
        </section>

        {!hasMessages && QUICK_ACTIONS.length > 0 && (
          <section
            className="flex flex-wrap justify-center gap-3 mt-8"
            aria-label="Quick actions"
          >
            {QUICK_ACTIONS.map((action, index) => (
              <QuickActionButton
                key={index}
                action={action}
                onAction={handleQuickAction}
              />
            ))}
          </section>
        )}

        {error && (
          <ErrorDisplay error={error} onRetry={clearError} />
        )}

        {isMinimizing && (
          <div className="flex justify-center items-center mt-4 p-3 bg-orange-500/20 border border-orange-500/30 rounded-lg text-orange-300 text-sm">
            <Loader2 className="h-4 w-4 animate-spin mr-2" />
            <span>Redirecting to full chat...</span>
          </div>
        )}

        {hasMessages && (
          <div className="flex justify-center mt-4">
            <Badge
              variant="secondary"
              className="bg-gray-800 text-gray-400 border border-gray-700"
            >
              {messages.length} message{messages.length !== 1 ? "s" : ""}
            </Badge>
          </div>
        )}
    </div>
  );
}