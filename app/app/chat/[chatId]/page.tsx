"use client";
import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import { useParams } from "next/navigation";
import { useAIChat } from "@/hooks/use-ai-chat";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Loader2, Send, MessageCircle, ChevronRight, Plus } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import Link from "next/link";
import { ChatMessage, AIThinkingIndicator, StreamingMessage } from "@/components/chat/chat-message";
import { InputGroup, InputGroupButton, InputGroupTextarea } from "@/components/ui/input-group";
import { AppPageHeader } from "@/components/app-page-header";

// Type for optimistic messages
interface OptimisticMessage {
  id: string;
  content: string;
  role: 'user' | 'assistant';
  created_at: string;
  isOptimistic?: boolean;
}

// Local storage message shape (from embedded-ai-chat)
interface LocalMsg {
  id: string;
  session_id: string;
  content: string;
  message_type?: string;
  role: 'user' | 'assistant' | string;
  created_at: string;
}


export default function ChatPage() {
  const params = useParams();
  const chatId = params.chatId as string;

  const {
    currentSession,
    messages,
    isLoading,
    isStreaming,
    error,
    sendStreamingMessage,
    createSession,
    loadSession,
    clearError,
  } = useAIChat();

  // State
  const [message, setMessage] = useState("");
  const [userScrolled, setUserScrolled] = useState(false);
  const [optimisticMessages, setOptimisticMessages] = useState<OptimisticMessage[]>([]);
  const [localMessages, setLocalMessages] = useState<OptimisticMessage[]>([]);
  // Tools are always on; we keep a fixed mode.
  const chatMode: "auto" | "agent" | "manual" = "agent";

  // Refs
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const messagesContainerRef = useRef<HTMLDivElement>(null);
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  const autoScrollTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  // Constants
  const MAX_MESSAGE_LENGTH = 4000;

  // Load session when chatId changes
  useEffect(() => {
    if (chatId && chatId !== "new" && chatId !== "undefined") {
      loadSession(chatId);
    }
  }, [chatId, loadSession]);

  // Hydrate local messages and chat mode saved by the embedded chat before redirect
  // Also check for pending message to send
  useEffect(() => {
    try {
      if (!chatId || chatId === "new" || chatId === "undefined") return;
      
      // Load messages
      const storageKey = `ai_chat:${chatId}:messages`;
      const raw = typeof window !== "undefined" ? window.localStorage.getItem(storageKey) : null;
      const parsed: unknown = raw ? JSON.parse(raw) : [];
      if (Array.isArray(parsed)) {
        const mapped = (parsed as LocalMsg[]).map((m) => ({
          id: m.id ?? `local-${Date.now()}`,
          content: m.content,
          role: (m.role === 'user' || m.role === 'assistant') ? m.role : 'user',
          created_at: m.created_at,
          isOptimistic: true,
        } satisfies OptimisticMessage));
        setLocalMessages(mapped);
      } else {
        setLocalMessages([]);
      }
      
      // Check for pending message to send
      const pendingMessageKey = `ai_chat:${chatId}:pending`;
      const pendingMessage = typeof window !== "undefined" ? window.localStorage.getItem(pendingMessageKey) : null;
      if (pendingMessage) {
        // Clear the pending message immediately
        if (typeof window !== "undefined") {
          window.localStorage.removeItem(pendingMessageKey);
        }
        
        // Send the message after a short delay to ensure session is loaded
        setTimeout(async () => {
          try {
            await sendStreamingMessage(pendingMessage, chatId, {
              use_tools: true,
              include_context: true,
              max_context_vectors: 10,
            });
          } catch (err) {
            console.error("Failed to send pending message:", err);
          }
        }, 500);
      }
    } catch {
      setLocalMessages([]);
    }
  }, [chatId, sendStreamingMessage]);

  // Combine local (hydrated) messages, real messages, and optimistic messages
  const allMessages = useMemo(
    () => [...localMessages, ...messages, ...optimisticMessages],
    [localMessages, messages, optimisticMessages]
  );

  // Enhanced scrolling function
  const scrollToBottom = useCallback((behavior: ScrollBehavior = "smooth", force: boolean = false) => {
    // Don't auto-scroll if user has manually scrolled up, unless forced
    if (!force && userScrolled && !isStreaming) {
      return;
    }

    if (messagesEndRef.current) {
      // Clear any existing timeout
      if (autoScrollTimeoutRef.current) {
        clearTimeout(autoScrollTimeoutRef.current);
      }

      // Use requestAnimationFrame for smoother scrolling
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

  // Detect user scroll to disable auto-scroll temporarily
  const handleScroll = useCallback(() => {
    const scrollArea = scrollAreaRef.current;
    if (!scrollArea) return;

    const { scrollTop, scrollHeight, clientHeight } = scrollArea;
    const isAtBottom = Math.abs(scrollHeight - clientHeight - scrollTop) < 50;
    
    // Update user scrolled state
    if (!isAtBottom && !isStreaming) {
      setUserScrolled(true);
    } else if (isAtBottom) {
      setUserScrolled(false);
    }
  }, [isStreaming]);

  // Enhanced auto-scroll during streaming with better throttling
  useEffect(() => {
    if (isStreaming) {
      // Clear any existing timeout
      if (autoScrollTimeoutRef.current) {
        clearTimeout(autoScrollTimeoutRef.current);
      }

      // Use a shorter throttle during streaming for more responsive scrolling
      autoScrollTimeoutRef.current = setTimeout(() => {
        scrollToBottom("smooth", true); // Force scroll during streaming
      }, 16); // ~60fps for smooth streaming

      return () => {
        if (autoScrollTimeoutRef.current) {
          clearTimeout(autoScrollTimeoutRef.current);
        }
      };
    }
  }, [isStreaming, scrollToBottom]);

  // Auto-scroll when streaming starts
  useEffect(() => {
    if (isStreaming) {
      setUserScrolled(false); // Reset user scroll state when streaming starts
      scrollToBottom("smooth", true);
    }
  }, [isStreaming, scrollToBottom]);

  // Regular scroll for message updates (non-streaming)
  useEffect(() => {
    if (!isStreaming && !userScrolled) {
      scrollToBottom("auto");
    }
  }, [allMessages, isLoading, isStreaming, userScrolled, scrollToBottom]);

  // Handle content height changes during streaming
  useEffect(() => {
    const scrollArea = scrollAreaRef.current;
    if (!scrollArea || !isStreaming) return;

    const observer = new ResizeObserver(() => {
      if (isStreaming && !userScrolled) {
        scrollToBottom("smooth", true);
      }
    });

    // Observe the messages container for height changes
    if (messagesContainerRef.current) {
      observer.observe(messagesContainerRef.current);
    }

    return () => {
      observer.disconnect();
    };
  }, [isStreaming, userScrolled, scrollToBottom]);

  // Focus input when component mounts
  useEffect(() => {
    if (inputRef.current && !isLoading && !isStreaming) {
      inputRef.current.focus();
    }
  }, [isLoading, isStreaming]);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (autoScrollTimeoutRef.current) {
        clearTimeout(autoScrollTimeoutRef.current);
      }
    };
  }, []);

  // Clear optimistic messages when real messages are updated
  useEffect(() => {
    // If we have real messages and optimistic messages, clear optimistic ones
    // This happens when the server responds with the actual saved messages
    if (messages.length > 0 && optimisticMessages.length > 0 && !isStreaming) {
      setOptimisticMessages([]);
    }
  }, [messages, optimisticMessages.length, isStreaming]);

  // Clear locally hydrated messages (and storage) once real messages arrive
  useEffect(() => {
    if (messages.length > 0 && localMessages.length > 0 && !isStreaming) {
      setLocalMessages([]);
      try {
        if (chatId && chatId !== "new" && chatId !== "undefined") {
          const storageKey = `ai_chat:${chatId}:messages`;
          if (typeof window !== "undefined") {
            window.localStorage.removeItem(storageKey);
          }
        }
      } catch {
        // ignore
      }
    }
  }, [messages.length, localMessages.length, isStreaming, chatId]);

  const handleSendMessage = useCallback(async () => {
    if (!message.trim() || message.trim().length > MAX_MESSAGE_LENGTH) return;

    const userMessage = message.trim();
    
    try {
      // Reset scroll state when sending a new message
      setUserScrolled(false);

      // Immediately add the user message to optimistic messages
      const optimisticUserMessage: OptimisticMessage = {
        id: `optimistic-${Date.now()}`,
        content: userMessage,
        role: 'user',
        created_at: new Date().toISOString(),
        isOptimistic: true
      };

      setOptimisticMessages(prev => [...prev, optimisticUserMessage]);
      
      // Clear the input immediately
      setMessage("");

      // Determine session ID
      let sessionId = chatId;
      if (chatId === "new" || chatId === "undefined") {
        // Create new session
        sessionId = await createSession("New Chat");
      }

      // Determine if we should use tools based on chat mode
      const useTools = chatMode === "agent";
      
      // Use streaming API for better UX with optional tool support
      await sendStreamingMessage(userMessage, sessionId, {
        use_tools: useTools,
        include_context: true,
        max_context_vectors: 10,
      });
      
    } catch (err) {
      console.error("Failed to send message:", err);
      // Remove the optimistic message on error
      setOptimisticMessages(prev => prev.filter(msg => msg.content !== userMessage));
      // Restore the message in the input
      setMessage(userMessage);
    }
  }, [message, chatId, createSession, sendStreamingMessage, chatMode]);

  const handleKeyPress = useCallback((e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  }, [handleSendMessage]);

  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const value = e.target.value;
    if (value.length <= MAX_MESSAGE_LENGTH) {
      setMessage(value);
    }
  }, []);

  const handleNewChat = useCallback(() => {
    // This would navigate to a new chat
    setUserScrolled(false);
    setOptimisticMessages([]); // Clear optimistic messages
    window.location.href = "/app/chat/new";
  }, []);

  const canSendMessage = !isLoading && !isStreaming && message.trim().length > 0;

  if (isLoading && !allMessages.length) {
    return (
      <div className="h-screen flex items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-orange-500" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-screen flex items-center justify-center">
        <div className="max-w-md p-6 rounded-lg border border-red-500/30 bg-red-500/10">
          <h2 className="text-xl font-bold text-red-500 mb-3">Error</h2>
          <p className="text-sm text-gray-300 mb-4">{error.message}</p>
          <Button onClick={clearError} variant="outline" className="border-red-500/50 text-red-400 hover:bg-red-500/20">
            Try Again
          </Button>
        </div>
      </div>
    );
  }

  // Get streaming content for display
  const streamingContent = isStreaming && messages.length > 0 
    ? messages[messages.length - 1]?.content 
    : "";

  // Filter out the last message if it's currently streaming
  const displayMessages = isStreaming && messages.length > 0
    ? allMessages.slice(0, -1)
    : allMessages;

  return (
    <div className="h-screen flex flex-col">
      {/* Header */}
      <AppPageHeader
        title={currentSession?.title || "AI Chat"}
        actions={
          <div className="flex items-center gap-3">
            <MessageCircle className="h-5 w-5" />
            {currentSession && (
              <Badge variant="secondary">
                {allMessages.length} messages
              </Badge>
            )}
            <Badge variant="default" className="bg-orange-500 hover:bg-orange-600">
              🔧 Tools Active
            </Badge>
          </div>
        }
      />

      {/* Chat Messages */}
      <div className="flex-1 overflow-hidden relative">
        <ScrollArea 
          className="h-full" 
          ref={scrollAreaRef}
          onScrollCapture={handleScroll}
        >
          <div className="max-w-5xl mx-auto px-8 py-8" ref={messagesContainerRef}>
            {displayMessages.length === 0 && !isLoading && !isStreaming ? (
              <div className="text-center text-gray-400 py-16">
                <MessageCircle className="h-16 w-16 mx-auto mb-6 opacity-30" />
                <p className="text-lg font-medium mb-2">Start a conversation with AI</p>
                <p className="text-sm">Ask about your trading data, strategies, or analysis</p>
              </div>
            ) : (
              <>
                {/* Render all completed messages */}
                {displayMessages.map((msg) => (
                  <ChatMessage 
                    key={msg.id} 
                    message={{
                      id: msg.id,
                      role: msg.role,
                      content: msg.content,
                      created_at: msg.created_at,
                      message_type: msg.role === 'user' ? 'user_question' : undefined
                    }} 
                  />
                ))}
                
                {/* Show AI thinking indicator when waiting for first response */}
                {isLoading && !isStreaming && <AIThinkingIndicator />}
                
                {/* Show streaming message with actual markdown content */}
                {isStreaming && streamingContent && (
                  <StreamingMessage content={streamingContent} />
                )}
              </>
            )}
            
            {/* Invisible div to mark the end of messages */}
            <div ref={messagesEndRef} className="h-1" data-messages-end />
          </div>
        </ScrollArea>
        
        {/* Show scroll indicator when user has scrolled up during streaming */}
        {userScrolled && isStreaming && (
          <div className="absolute bottom-8 left-1/2 transform -translate-x-1/2 z-10">
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                setUserScrolled(false);
                scrollToBottom("smooth", true);
              }}
              className="bg-orange-500/90 border-orange-500/30 text-white hover:bg-orange-600 shadow-lg backdrop-blur-sm"
            >
              New messages ↓
            </Button>
          </div>
        )}
      </div>

      {/* Message Input */}
      <div className="px-4 pb-4 flex-shrink-0">
        <div className="max-w-2xl mx-auto">
          <InputGroup className="bg-gray-900 border-gray-700 rounded-2xl">
            <InputGroupTextarea
              ref={inputRef}
              placeholder="How can I help you today?"
              value={message}
              onChange={handleInputChange}
              onKeyDown={handleKeyPress}
              disabled={isLoading || isStreaming}
              maxLength={MAX_MESSAGE_LENGTH}
              className="text-white placeholder:text-gray-300 font-medium"
              aria-label="Type your message"
              rows={4}
            />
            <div className="flex items-center gap-2 pr-2">
              <InputGroupButton
              variant="ghost"
                size="icon-sm"
              onClick={handleNewChat}
              disabled={isLoading || isStreaming}
              aria-label="Start new chat"
            >
              <Plus className="h-4 w-4" />
              </InputGroupButton>
              <InputGroupButton
                variant="default"
                size="icon-sm"
              onClick={handleSendMessage}
              disabled={!canSendMessage}
              aria-label="Send message"
                className="rounded-full"
            >
              {isLoading || isStreaming ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Send className="h-4 w-4" />
              )}
              </InputGroupButton>
            </div>
          </InputGroup>
          {/* Character count */}
          {message.length > MAX_MESSAGE_LENGTH * 0.8 && (
            <div className="flex justify-end text-xs text-gray-400 mt-2 pr-1">
              {message.length}/{MAX_MESSAGE_LENGTH}
          </div>
          )}
        </div>
      </div>
    </div>
  );
}