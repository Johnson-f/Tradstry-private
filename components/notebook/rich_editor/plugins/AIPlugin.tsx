'use client';

import { useEffect, useState, useCallback, useRef } from 'react';
import { useLexicalComposerContext } from '@lexical/react/LexicalComposerContext';
import {
  $getSelection,
  $isRangeSelection,
  COMMAND_PRIORITY_LOW,
  KEY_ESCAPE_COMMAND,
} from 'lexical';
import { createPortal } from 'react-dom';
import {
  Sparkles,
  SpellCheck,
  FileText,
  Minimize2,
  Maximize2,
  Wand2,
  Languages,
  CheckCircle,
  Loader2,
  X,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { useWs } from '@/lib/websocket/provider';
import {
  notebookAIService,
  generateRequestId,
  type NotebookAIAction,
  type NotebookAIChunkEvent,
  type NotebookAICompleteEvent,
} from '@/lib/services/notebook-ai-service';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';

interface AIPluginProps {
  noteId?: string;
}

interface PendingRequest {
  requestId: string;
  action: NotebookAIAction;
  originalContent: string;
  result: string;
  isComplete: boolean;
}

export function AIPlugin({ noteId }: AIPluginProps) {
  const [editor] = useLexicalComposerContext();
  const { subscribe } = useWs();
  const [showToolbar, setShowToolbar] = useState(false);
  const [toolbarPosition, setToolbarPosition] = useState({ top: 0, left: 0 });
  const [selectedText, setSelectedText] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [pendingRequest, setPendingRequest] = useState<PendingRequest | null>(null);
  const [showPreview, setShowPreview] = useState(false);
  const toolbarRef = useRef<HTMLDivElement>(null);

  // Handle selection changes
  useEffect(() => {
    const updateToolbar = () => {
      editor.getEditorState().read(() => {
        const selection = $getSelection();
        if ($isRangeSelection(selection) && !selection.isCollapsed()) {
          const text = selection.getTextContent();
          if (text.trim().length > 0) {
            setSelectedText(text);
            
            // Get selection position
            const domSelection = window.getSelection();
            if (domSelection && domSelection.rangeCount > 0) {
              const range = domSelection.getRangeAt(0);
              const rect = range.getBoundingClientRect();
              setToolbarPosition({
                top: rect.top - 45,
                left: rect.left + rect.width / 2,
              });
              setShowToolbar(true);
            }
          } else {
            setShowToolbar(false);
          }
        } else {
          setShowToolbar(false);
        }
      });
    };

    const removeListener = editor.registerUpdateListener(({ editorState }) => {
      editorState.read(() => {
        updateToolbar();
      });
    });

    return () => removeListener();
  }, [editor]);

  // Handle escape key to close toolbar
  useEffect(() => {
    return editor.registerCommand(
      KEY_ESCAPE_COMMAND,
      () => {
        if (showToolbar || showPreview) {
          setShowToolbar(false);
          setShowPreview(false);
          setPendingRequest(null);
          return true;
        }
        return false;
      },
      COMMAND_PRIORITY_LOW
    );
  }, [editor, showToolbar, showPreview]);

  // Subscribe to WebSocket events
  useEffect(() => {
    const handleChunk = (data: NotebookAIChunkEvent) => {
      if (pendingRequest && data.request_id === pendingRequest.requestId) {
        setPendingRequest((prev) =>
          prev
            ? {
                ...prev,
                result: prev.result + data.chunk,
                isComplete: data.is_complete,
              }
            : null
        );
        if (data.is_complete) {
          setIsProcessing(false);
        }
      }
    };

    const handleComplete = (data: NotebookAICompleteEvent) => {
      if (pendingRequest && data.request_id === pendingRequest.requestId) {
        setPendingRequest((prev) =>
          prev
            ? {
                ...prev,
                result: data.result,
                isComplete: true,
              }
            : null
        );
        setIsProcessing(false);
        setShowPreview(true);
      }
    };

    // Subscribe with normalized event names (colons, not underscores)
    const unsubChunk = subscribe('notebook:ai:chunk', handleChunk as (data: unknown) => void);
    const unsubComplete = subscribe('notebook:ai:complete', handleComplete as (data: unknown) => void);

    return () => {
      unsubChunk();
      unsubComplete();
    };
  }, [subscribe, pendingRequest]);

  // Process AI action
  const handleAIAction = useCallback(
    async (action: NotebookAIAction) => {
      if (!selectedText.trim()) return;

      setIsProcessing(true);
      const requestId = generateRequestId();

      setPendingRequest({
        requestId,
        action,
        originalContent: selectedText,
        result: '',
        isComplete: false,
      });

      try {
        await notebookAIService.process({
          action,
          content: selectedText,
          note_id: noteId,
          request_id: requestId,
        });
      } catch (error) {
        console.error('AI request failed:', error);
        toast.error('Failed to process AI request');
        setIsProcessing(false);
        setPendingRequest(null);
      }
    },
    [selectedText, noteId]
  );

  // Apply AI result to editor
  const applyResult = useCallback(() => {
    if (!pendingRequest?.result) return;

    editor.update(() => {
      const selection = $getSelection();
      if ($isRangeSelection(selection)) {
        selection.insertText(pendingRequest.result);
      }
    });

    toast.success('AI suggestion applied');
    setShowPreview(false);
    setShowToolbar(false);
    setPendingRequest(null);
  }, [editor, pendingRequest]);

  // Discard AI result
  const discardResult = useCallback(() => {
    setShowPreview(false);
    setPendingRequest(null);
    toast.info('AI suggestion discarded');
  }, []);

  const actionItems = [
    { action: 'correct_spelling' as NotebookAIAction, label: 'Fix Spelling', icon: SpellCheck },
    { action: 'fix_grammar' as NotebookAIAction, label: 'Fix Grammar', icon: CheckCircle },
    { action: 'improve_writing' as NotebookAIAction, label: 'Improve Writing', icon: Wand2 },
    { action: 'summarize' as NotebookAIAction, label: 'Summarize', icon: FileText },
    { action: 'make_shorter' as NotebookAIAction, label: 'Make Shorter', icon: Minimize2 },
    { action: 'make_longer' as NotebookAIAction, label: 'Make Longer', icon: Maximize2 },
    { action: 'simplify_language' as NotebookAIAction, label: 'Simplify', icon: Languages },
  ];

  // Render toolbar
  const renderToolbar = () => {
    if (!showToolbar || typeof window === 'undefined') return null;

    return createPortal(
      <div
        ref={toolbarRef}
        className={cn(
          'fixed z-50 flex items-center gap-1 rounded-lg border bg-popover p-1 shadow-lg',
          'animate-in fade-in-0 zoom-in-95'
        )}
        style={{
          top: toolbarPosition.top,
          left: toolbarPosition.left,
          transform: 'translateX(-50%)',
        }}
      >
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="sm"
              className="h-8 gap-1.5 px-2"
              disabled={isProcessing}
            >
              {isProcessing ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Sparkles className="h-4 w-4" />
              )}
              <span className="text-xs">AI</span>
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start" className="w-48">
            {actionItems.map((item) => (
              <DropdownMenuItem
                key={item.action}
                onClick={() => handleAIAction(item.action)}
                disabled={isProcessing}
              >
                <item.icon className="mr-2 h-4 w-4" />
                {item.label}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>,
      document.body
    );
  };

  // Render preview modal
  const renderPreview = () => {
    if (!showPreview || !pendingRequest || typeof window === 'undefined') return null;

    return createPortal(
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
        <div className="mx-4 w-full max-w-2xl rounded-lg border bg-background p-6 shadow-xl">
          <div className="mb-4 flex items-center justify-between">
            <h3 className="text-lg font-semibold">AI Suggestion</h3>
            <Button variant="ghost" size="icon" onClick={discardResult}>
              <X className="h-4 w-4" />
            </Button>
          </div>

          <div className="mb-4 space-y-4">
            <div>
              <p className="mb-1 text-sm font-medium text-muted-foreground">Original</p>
              <div className="rounded-md bg-muted p-3 text-sm">
                {pendingRequest.originalContent}
              </div>
            </div>

            <div>
              <p className="mb-1 text-sm font-medium text-muted-foreground">Suggestion</p>
              <div className="rounded-md border border-primary/20 bg-primary/5 p-3 text-sm">
                {pendingRequest.result || (
                  <span className="flex items-center gap-2 text-muted-foreground">
                    <Loader2 className="h-4 w-4 animate-spin" />
                    Generating...
                  </span>
                )}
              </div>
            </div>
          </div>

          <div className="flex justify-end gap-2">
            <Button variant="outline" onClick={discardResult}>
              Discard
            </Button>
            <Button
              onClick={applyResult}
              disabled={!pendingRequest.isComplete || !pendingRequest.result}
            >
              Apply
            </Button>
          </div>
        </div>
      </div>,
      document.body
    );
  };

  return (
    <>
      {renderToolbar()}
      {renderPreview()}
    </>
  );
}
