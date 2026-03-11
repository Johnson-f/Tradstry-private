'use client';

import { useMemo, type ComponentType, type ReactNode } from 'react';
import type { InitialConfigType } from '@lexical/react/LexicalComposer';
import { LexicalComposer } from '@lexical/react/LexicalComposer';
import { RichTextPlugin } from '@lexical/react/LexicalRichTextPlugin';
import { ContentEditable } from '@lexical/react/LexicalContentEditable';
import { LexicalErrorBoundary } from '@lexical/react/LexicalErrorBoundary';
import { HistoryPlugin } from '@lexical/react/LexicalHistoryPlugin';
import { OnChangePlugin } from '@lexical/react/LexicalOnChangePlugin';
import { ListPlugin } from '@lexical/react/LexicalListPlugin';
import { LinkPlugin } from '@lexical/react/LexicalLinkPlugin';
import { TablePlugin } from '@lexical/react/LexicalTablePlugin';
import { MarkdownShortcutPlugin } from '@lexical/react/LexicalMarkdownShortcutPlugin';
import { HeadingNode, QuoteNode } from '@lexical/rich-text';
import { ScrollArea } from '@/components/ui/scroll-area';
import { ListItemNode, ListNode } from '@lexical/list';
import { AutoLinkNode, LinkNode } from '@lexical/link';
import { CodeNode, CodeHighlightNode } from '@lexical/code';
import { TableNode, TableCellNode, TableRowNode } from '@lexical/table';
import { $getSelection, EditorState } from 'lexical';
import { TRANSFORMERS } from '@lexical/markdown';
import { notebookEditorTheme } from './theme';
import { SlashCommands } from './slash_commands';
import {
  AIPlugin,
  AIAutocompletePlugin,
  CodeHighlightPlugin,
  CodeLanguageSelectPlugin,
  ImagePlugin,
  TableActionMenuPlugin,
} from './plugins';
import { ImageNode } from './nodes/ImageNode';
import { YouTubeNode } from './nodes/YouTubeNode';
import { TweetNode } from './nodes/TweetNode';

type RichEditorProps = {
  value?: string;
  placeholder?: string;
  onChange?: (serializedState: string) => void;
  className?: string;
  title?: string;
  onTitleChange?: (title: string) => void;
  titlePlaceholder?: string;
  noteId?: string;
};

const editorNodes = [
  HeadingNode,
  QuoteNode,
  ListNode,
  ListItemNode,
  CodeNode,
  CodeHighlightNode,
  LinkNode,
  AutoLinkNode,
  ImageNode,
  YouTubeNode,
  TweetNode,
  TableNode,
  TableCellNode,
  TableRowNode,
];

const onError = (error: Error) => {
  console.error('Lexical editor error:', error);
};

// Default empty editor state with a valid root node containing a paragraph
const DEFAULT_EDITOR_STATE = JSON.stringify({
  root: {
    children: [
      {
        children: [],
        direction: null,
        format: "",
        indent: 0,
        type: "paragraph",
        version: 1,
      },
    ],
    direction: null,
    format: "",
    indent: 0,
    type: "root",
    version: 1,
  },
});

/**
 * Validates if the editor state JSON has a valid structure
 */
function isValidEditorState(value: string): boolean {
  try {
    const parsed = JSON.parse(value);
    // Check if root exists and has children array with at least one element
    if (!parsed?.root?.children || !Array.isArray(parsed.root.children)) {
      return false;
    }
    // Empty children array is invalid - needs at least one paragraph
    if (parsed.root.children.length === 0) {
      return false;
    }
    return true;
  } catch {
    return false;
  }
}

function EditorErrorBoundary({ children }: { children: ReactNode }) {
  const LexicalErrorBoundaryComponent = LexicalErrorBoundary as unknown as ComponentType<{
    children: ReactNode;
  }>;

  return <LexicalErrorBoundaryComponent>{children}</LexicalErrorBoundaryComponent>;
}

export function RichEditor({
  value,
  placeholder = 'Start writing…',
  onChange,
  className,
  title = '',
  onTitleChange,
  titlePlaceholder = 'New page',
  noteId,
}: RichEditorProps) {
  // Determine the editor state to use - validate and fallback to default if invalid
  const validatedValue = useMemo(() => {
    if (!value) return undefined;
    if (isValidEditorState(value)) return value;
    // If invalid, use default state
    console.warn('Invalid editor state provided, using default');
    return DEFAULT_EDITOR_STATE;
  }, [value]);

  const initialConfig: InitialConfigType = useMemo(
    () => ({
      namespace: 'notebook-editor',
      theme: notebookEditorTheme,
      nodes: editorNodes,
      editorState: validatedValue
        ? (editor) => {
            try {
              const parsed = editor.parseEditorState(validatedValue);
              editor.setEditorState(parsed);
            } catch (e) {
              console.error('Failed to parse editor state:', e);
              // Fallback to default state
              const defaultParsed = editor.parseEditorState(DEFAULT_EDITOR_STATE);
              editor.setEditorState(defaultParsed);
            }
          }
        : undefined,
      onError,
    }),
    [validatedValue],
  );

  return (
    <LexicalComposer initialConfig={initialConfig}>
      <div className={`${className} relative h-full`}>
        <ScrollArea className="h-full">
          <div className="relative">
            {/* Title Input Section */}
            <div className="pl-16 pr-8 pt-12 pb-2">
              <input
                type="text"
                value={title}
                onChange={(e) => onTitleChange?.(e.target.value)}
                placeholder={titlePlaceholder}
                className="w-full bg-transparent text-4xl font-bold outline-none placeholder:text-muted-foreground/40"
                style={{ caretColor: 'currentColor' }}
              />
            </div>

            {/* Rich Text Editor */}
            <div className="relative mt-2">
              <RichTextPlugin
                contentEditable={
                  <ContentEditable className="min-h-[200px] pl-16 pr-8 py-6 outline-none" />
                }
                placeholder={
                  <div className="pointer-events-none absolute left-16 top-6 select-none text-sm text-muted-foreground">
                    {placeholder}
                  </div>
                }
                ErrorBoundary={EditorErrorBoundary}
              />
              <HistoryPlugin />
              <ListPlugin />
              <LinkPlugin />
              <MarkdownShortcutPlugin transformers={TRANSFORMERS} />
              <SlashCommands />
              <CodeLanguageSelectPlugin />
              <CodeHighlightPlugin />
              <ImagePlugin noteId={noteId} />
              <TablePlugin />
              <TableActionMenuPlugin />
              <AIPlugin noteId={noteId} />
              <AIAutocompletePlugin noteId={noteId} enabled={true} debounceMs={1500} />
              <OnChangePlugin
                onChange={(editorState: EditorState) => {
                  if (!onChange) {
                    return;
                  }

                  const serialized = editorState.toJSON();
                  editorState.read(() => {
                    $getSelection();
                  });

                  onChange(JSON.stringify(serialized));
                }}
              />
            </div>
          </div>
        </ScrollArea>
      </div>
    </LexicalComposer>
  );
}
