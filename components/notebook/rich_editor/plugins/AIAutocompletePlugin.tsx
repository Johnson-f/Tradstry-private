'use client';

import { useEffect, useState, useCallback, useRef } from 'react';
import { useLexicalComposerContext } from '@lexical/react/LexicalComposerContext';
import {
  $getSelection,
  $isRangeSelection,
  KEY_TAB_COMMAND,
  KEY_ESCAPE_COMMAND,
  COMMAND_PRIORITY_HIGH,
} from 'lexical';
import { createPortal } from 'react-dom';
import { useWs } from '@/lib/websocket/provider';
import {
  notebookAIService,
  generateRequestId,
  type NotebookAIChunkEvent,
  type NotebookAICompleteEvent,
} from '@/lib/services/notebook-ai-service';
import { cn } from '@/lib/utils';
import { Sparkles, Loader2 } from 'lucide-react';

interface AIAutocompletePluginProps {
  noteId?: string;
  enabled?: boolean;
  debounceMs?: number;
}

export function AIAutocompletePlugin({
  noteId,
  enabled = true,
  debounceMs = 1500,
}: AIAutocompletePluginProps) {
  const [editor] = useLexicalComposerContext();
  const { subscribe } = useWs();
  const [suggestion, setSuggestion] = useState('');
  const [suggestionPosition, setSuggestionPosition] = useState({ top: 0, left: 0 });
  const [isLoading, setIsLoading] = useState(false);
  const [currentRequestId, setCurrentRequestId] = useState<string | null>(null);
  const [isVisible, setIsVisible] = useState(false);
  const debounceTimerRef = useRef<NodeJS.Timeout | null>(null);
  const lastTextRef = useRef<string>('');
  const popoverRef = useRef<HTMLDivElement>(null);

  // Get current paragraph text for context
  const getCurrentParagraphText = useCallback(() => {
    let text = '';
    editor.getEditorState().read(() => {
      const selection = $getSelection();
      if ($isRangeSelection(selection)) {
        const anchorNode = selection.anchor.getNode();
        const paragraph = anchorNode.getTopLevelElement();
        if (paragraph) {
          text = paragraph.getTextContent();
        }
      }
    });
    return text;
  }, [editor]);

  // Get cursor position for suggestion display
  const getCursorPosition = useCallback(() => {
    const domSelection = window.getSelection();
    if (domSelection && domSelection.rangeCount > 0) {
      const range = domSelection.getRangeAt(0);
      const rect = range.getBoundingClientRect();
      return { top: rect.bottom + 8, left: rect.left };
    }
    return { top: 0, left: 0 };
  }, []);

  // Request autocomplete suggestion
  const requestAutocomplete = useCallback(async (text: string) => {
    if (!text.trim() || text.length < 10) return;

    setIsLoading(true);
    setIsVisible(true);
    const requestId = generateRequestId();
    setCurrentRequestId(requestId);
    setSuggestion('');

    try {
      await notebookAIService.process({
        action: 'autocomplete',
        content: text,
        note_id: noteId,
        request_id: requestId,
      });
    } catch (error) {
      console.error('Autocomplete request failed:', error);
      setIsLoading(false);
      setIsVisible(false);
      setCurrentRequestId(null);
    }
  }, [noteId]);

  // Handle text changes with debounce
  useEffect(() => {
    if (!enabled) return;

    const removeListener = editor.registerUpdateListener(({ editorState, tags }) => {
      // Only trigger on user input, not programmatic changes
      if (tags.has('history-merge') || tags.has('collaboration')) return;

      editorState.read(() => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection) || !selection.isCollapsed()) {
          setSuggestion('');
          setIsVisible(false);
          return;
        }

        const text = getCurrentParagraphText();
        
        // Clear existing timer
        if (debounceTimerRef.current) {
          clearTimeout(debounceTimerRef.current);
        }

        // Clear suggestion if text changed significantly
        if (text !== lastTextRef.current) {
          setSuggestion('');
          setIsVisible(false);
          lastTextRef.current = text;
        }

        // Set new debounce timer
        debounceTimerRef.current = setTimeout(() => {
          const currentText = getCurrentParagraphText();
          if (currentText.trim().length >= 10) {
            setSuggestionPosition(getCursorPosition());
            requestAutocomplete(currentText);
          }
        }, debounceMs);
      });
    });

    return () => {
      removeListener();
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
      }
    };
  }, [editor, enabled, debounceMs, getCurrentParagraphText, getCursorPosition, requestAutocomplete]);

  // Subscribe to WebSocket events
  useEffect(() => {
    const handleChunk = (data: NotebookAIChunkEvent) => {
      if (currentRequestId && data.request_id === currentRequestId) {
        setSuggestion((prev) => prev + data.chunk);
        if (data.is_complete) {
          setIsLoading(false);
        }
      }
    };

    const handleComplete = (data: NotebookAICompleteEvent) => {
      if (currentRequestId && data.request_id === currentRequestId) {
        setSuggestion(data.result);
        setIsLoading(false);
      }
    };

    // Subscribe with normalized event names (colons, not underscores)
    const unsubChunk = subscribe('notebook:ai:chunk', handleChunk as (data: unknown) => void);
    const unsubComplete = subscribe('notebook:ai:complete', handleComplete as (data: unknown) => void);

    return () => {
      unsubChunk();
      unsubComplete();
    };
  }, [subscribe, currentRequestId]);

  // Accept suggestion
  const acceptSuggestion = useCallback(() => {
    if (!suggestion) return;
    
    editor.update(() => {
      const selection = $getSelection();
      if ($isRangeSelection(selection)) {
        selection.insertText(suggestion);
      }
    });
    setSuggestion('');
    setIsVisible(false);
    setCurrentRequestId(null);
  }, [editor, suggestion]);

  // Dismiss suggestion
  const dismissSuggestion = useCallback(() => {
    setSuggestion('');
    setIsVisible(false);
    setCurrentRequestId(null);
  }, []);

  // Handle Tab key to accept suggestion
  useEffect(() => {
    return editor.registerCommand(
      KEY_TAB_COMMAND,
      (event) => {
        if (suggestion && isVisible) {
          event.preventDefault();
          acceptSuggestion();
          return true;
        }
        return false;
      },
      COMMAND_PRIORITY_HIGH
    );
  }, [editor, suggestion, isVisible, acceptSuggestion]);

  // Handle Escape key to dismiss suggestion
  useEffect(() => {
    return editor.registerCommand(
      KEY_ESCAPE_COMMAND,
      () => {
        if (isVisible) {
          dismissSuggestion();
          return true;
        }
        return false;
      },
      COMMAND_PRIORITY_HIGH
    );
  }, [editor, isVisible, dismissSuggestion]);

  // Don't render if not visible or SSR
  if (!isVisible || typeof window === 'undefined') return null;

  return createPortal(
    <div
      ref={popoverRef}
      className={cn(
        'fixed z-50 animate-in fade-in-0 zoom-in-95 slide-in-from-top-2',
        'duration-200'
      )}
      style={{
        top: suggestionPosition.top,
        left: suggestionPosition.left,
      }}
    >
      {/* Main suggestion card */}
      <div
        className={cn(
          'min-w-[280px] max-w-[420px] overflow-hidden rounded-lg border',
          'bg-popover text-popover-foreground shadow-lg',
          'ring-1 ring-black/5 dark:ring-white/10'
        )}
      >
        {/* Header */}
        <div className="flex items-center gap-2 border-b bg-muted/50 px-3 py-2">
          <Sparkles className="h-3.5 w-3.5 text-primary" />
          <span className="text-xs font-medium text-muted-foreground">
            AI Suggestion
          </span>
          {isLoading && (
            <Loader2 className="ml-auto h-3 w-3 animate-spin text-muted-foreground" />
          )}
        </div>

        {/* Suggestion content */}
        <div className="p-3">
          {isLoading && !suggestion ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <span>Thinking</span>
              <span className="inline-flex">
                <span className="animate-pulse">.</span>
                <span className="animate-pulse delay-100">.</span>
                <span className="animate-pulse delay-200">.</span>
              </span>
            </div>
          ) : (
            <div className="relative">
              <p className="text-sm leading-relaxed text-foreground">
                {suggestion}
                {/* Blinking cursor */}
                <span
                  className={cn(
                    'ml-0.5 inline-block h-4 w-0.5 bg-primary',
                    'animate-pulse'
                  )}
                  style={{
                    animation: 'blink 1s step-end infinite',
                  }}
                />
              </p>
            </div>
          )}
        </div>

        {/* Footer with keyboard hint */}
        <div className="flex items-center justify-between border-t bg-muted/30 px-3 py-1.5">
          <div className="flex items-center gap-3">
            <button
              onClick={acceptSuggestion}
              disabled={!suggestion || isLoading}
              className={cn(
                'flex items-center gap-1.5 rounded px-2 py-1 text-xs',
                'bg-primary/10 text-primary hover:bg-primary/20',
                'transition-colors disabled:opacity-50 disabled:cursor-not-allowed'
              )}
            >
              <kbd className="rounded bg-primary/20 px-1 py-0.5 font-mono text-[10px]">
                Tab
              </kbd>
              <span>Accept</span>
            </button>
            <button
              onClick={dismissSuggestion}
              className={cn(
                'flex items-center gap-1.5 rounded px-2 py-1 text-xs',
                'text-muted-foreground hover:bg-muted',
                'transition-colors'
              )}
            >
              <kbd className="rounded bg-muted px-1 py-0.5 font-mono text-[10px]">
                Esc
              </kbd>
              <span>Dismiss</span>
            </button>
          </div>
        </div>
      </div>

      {/* CSS for blinking cursor */}
      <style jsx>{`
        @keyframes blink {
          0%, 50% { opacity: 1; }
          51%, 100% { opacity: 0; }
        }
      `}</style>
    </div>,
    document.body
  );
}
