'use client';

import { useCallback, useRef, useEffect, useState } from 'react';
import { useLexicalComposerContext } from '@lexical/react/LexicalComposerContext';
import { useLexicalNodeSelection } from '@lexical/react/useLexicalNodeSelection';
import { mergeRegister } from '@lexical/utils';
import {
  $getNodeByKey,
  $getSelection,
  $isNodeSelection,
  CLICK_COMMAND,
  COMMAND_PRIORITY_LOW,
  KEY_BACKSPACE_COMMAND,
  KEY_DELETE_COMMAND,
  NodeKey,
} from 'lexical';
import { cn } from '@/lib/utils';
import { X, ExternalLink, Twitter } from 'lucide-react';
import { Button } from '@/components/ui/button';

interface TweetComponentProps {
  tweetId: string;
  nodeKey: NodeKey;
}

// Declare Twitter widgets type
declare global {
  interface Window {
    twttr?: {
      widgets: {
        createTweet: (
          tweetId: string,
          container: HTMLElement,
          options?: Record<string, unknown>
        ) => Promise<HTMLElement>;
      };
    };
  }
}

export default function TweetComponent({ tweetId, nodeKey }: TweetComponentProps) {
  const [editor] = useLexicalComposerContext();
  const [isSelected, setSelected, clearSelection] = useLexicalNodeSelection(nodeKey);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const tweetContainerRef = useRef<HTMLDivElement>(null);

  const onDelete = useCallback(
    (event: KeyboardEvent) => {
      if (isSelected && $isNodeSelection($getSelection())) {
        event.preventDefault();
        editor.update(() => {
          const node = $getNodeByKey(nodeKey);
          if (node) {
            node.remove();
          }
        });
        return true;
      }
      return false;
    },
    [editor, isSelected, nodeKey]
  );

  const onClick = useCallback(
    (event: MouseEvent) => {
      if (containerRef.current?.contains(event.target as Node)) {
        if (!event.shiftKey) {
          clearSelection();
        }
        setSelected(true);
        return true;
      }
      return false;
    },
    [clearSelection, setSelected]
  );

  const handleRemove = useCallback(() => {
    editor.update(() => {
      const node = $getNodeByKey(nodeKey);
      if (node) {
        node.remove();
      }
    });
  }, [editor, nodeKey]);

  // Load Twitter widget script and render tweet
  useEffect(() => {
    const loadTwitterWidget = async () => {
      // Load Twitter widget script if not already loaded
      if (!window.twttr) {
        const script = document.createElement('script');
        script.src = 'https://platform.twitter.com/widgets.js';
        script.async = true;
        script.onload = () => renderTweet();
        script.onerror = () => {
          setError(true);
          setIsLoading(false);
        };
        document.body.appendChild(script);
      } else {
        renderTweet();
      }
    };

    const renderTweet = async () => {
      if (!tweetContainerRef.current || !window.twttr) {
        // Retry after a short delay if twttr isn't ready
        setTimeout(renderTweet, 100);
        return;
      }

      try {
        // Clear previous content
        tweetContainerRef.current.innerHTML = '';
        
        await window.twttr.widgets.createTweet(tweetId, tweetContainerRef.current, {
          theme: document.documentElement.classList.contains('dark') ? 'dark' : 'light',
          align: 'center',
          conversation: 'none',
        });
        setIsLoading(false);
      } catch (err) {
        console.error('Failed to load tweet:', err);
        setError(true);
        setIsLoading(false);
      }
    };

    loadTwitterWidget();
  }, [tweetId]);

  useEffect(() => {
    return mergeRegister(
      editor.registerCommand(CLICK_COMMAND, onClick, COMMAND_PRIORITY_LOW),
      editor.registerCommand(KEY_DELETE_COMMAND, onDelete, COMMAND_PRIORITY_LOW),
      editor.registerCommand(KEY_BACKSPACE_COMMAND, onDelete, COMMAND_PRIORITY_LOW)
    );
  }, [editor, onClick, onDelete]);

  return (
    <div
      ref={containerRef}
      className={cn(
        'relative mx-auto w-full max-w-[550px] overflow-hidden rounded-lg',
        'transition-all duration-200',
        isSelected && 'ring-2 ring-primary ring-offset-2'
      )}
    >
      {/* Tweet container */}
      <div ref={tweetContainerRef} className="min-h-[100px]">
        {isLoading && (
          <div className="flex h-[200px] items-center justify-center rounded-lg border bg-muted/50">
            <div className="flex flex-col items-center gap-2 text-muted-foreground">
              <Twitter className="h-8 w-8 animate-pulse" />
              <span className="text-sm">Loading tweet...</span>
            </div>
          </div>
        )}
        {error && (
          <div className="flex h-[150px] items-center justify-center rounded-lg border border-destructive/20 bg-destructive/5">
            <div className="flex flex-col items-center gap-2 text-muted-foreground">
              <Twitter className="h-8 w-8" />
              <span className="text-sm">Failed to load tweet</span>
              <a
                href={`https://twitter.com/i/status/${tweetId}`}
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs text-primary hover:underline"
              >
                View on Twitter
              </a>
            </div>
          </div>
        )}
      </div>

      {/* Controls overlay - visible on selection */}
      {isSelected && (
        <div className="absolute right-2 top-2 flex gap-1">
          <Button
            variant="secondary"
            size="icon"
            className="h-7 w-7 bg-background/80 backdrop-blur-sm"
            onClick={() => window.open(`https://twitter.com/i/status/${tweetId}`, '_blank')}
          >
            <ExternalLink className="h-3.5 w-3.5" />
          </Button>
          <Button
            variant="destructive"
            size="icon"
            className="h-7 w-7"
            onClick={handleRemove}
          >
            <X className="h-3.5 w-3.5" />
          </Button>
        </div>
      )}
    </div>
  );
}
