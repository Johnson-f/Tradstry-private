'use client';

import { useMemo } from 'react';
import Link from 'next/link';
import { ArrowLeft, Calendar, Eye, FileText } from 'lucide-react';
import { LexicalComposer } from '@lexical/react/LexicalComposer';
import { RichTextPlugin } from '@lexical/react/LexicalRichTextPlugin';
import { ContentEditable } from '@lexical/react/LexicalContentEditable';
import { LexicalErrorBoundary } from '@lexical/react/LexicalErrorBoundary';
import { HeadingNode, QuoteNode } from '@lexical/rich-text';
import { ListItemNode, ListNode } from '@lexical/list';
import { AutoLinkNode, LinkNode } from '@lexical/link';
import { CodeNode, CodeHighlightNode } from '@lexical/code';
import { TableNode, TableCellNode, TableRowNode } from '@lexical/table';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';

interface PublicNote {
  id: string;
  title: string;
  content: Record<string, unknown>;
  content_plain_text?: string;
  word_count: number;
  created_at: string;
  updated_at: string;
  view_count?: number;
  owner_name?: string;
}

interface PublicNoteViewerProps {
  note: PublicNote;
}

const editorNodes = [
  HeadingNode,
  QuoteNode,
  ListNode,
  ListItemNode,
  CodeNode,
  CodeHighlightNode,
  LinkNode,
  AutoLinkNode,
  TableNode,
  TableCellNode,
  TableRowNode,
];

const theme = {
  paragraph: 'mb-2 leading-relaxed',
  heading: {
    h1: 'text-3xl font-bold mb-4 mt-6',
    h2: 'text-2xl font-semibold mb-3 mt-5',
    h3: 'text-xl font-medium mb-2 mt-4',
  },
  list: {
    ul: 'list-disc ml-6 mb-4',
    ol: 'list-decimal ml-6 mb-4',
    listitem: 'mb-1',
  },
  quote: 'border-l-4 border-primary/30 pl-4 italic text-muted-foreground my-4',
  code: 'bg-muted px-1.5 py-0.5 rounded font-mono text-sm',
  codeHighlight: {
    atrule: 'text-purple-500',
    attr: 'text-yellow-500',
    boolean: 'text-purple-500',
    builtin: 'text-cyan-500',
    cdata: 'text-gray-500',
    char: 'text-green-500',
    class: 'text-yellow-500',
    'class-name': 'text-yellow-500',
    comment: 'text-gray-500',
    constant: 'text-purple-500',
    deleted: 'text-red-500',
    doctype: 'text-gray-500',
    entity: 'text-red-500',
    function: 'text-blue-500',
    important: 'text-purple-500',
    inserted: 'text-green-500',
    keyword: 'text-purple-500',
    namespace: 'text-red-500',
    number: 'text-green-500',
    operator: 'text-gray-500',
    prolog: 'text-gray-500',
    property: 'text-blue-500',
    punctuation: 'text-gray-500',
    regex: 'text-red-500',
    selector: 'text-green-500',
    string: 'text-green-500',
    symbol: 'text-purple-500',
    tag: 'text-red-500',
    url: 'text-cyan-500',
    variable: 'text-red-500',
  },
  link: 'text-primary underline hover:no-underline',
  table: 'border-collapse w-full my-4',
  tableCell: 'border border-border p-2',
  tableCellHeader: 'border border-border p-2 bg-muted font-semibold',
};

function formatDate(dateString: string) {
  return new Date(dateString).toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
}

export function PublicNoteViewer({ note }: PublicNoteViewerProps) {
  const initialConfig = useMemo(
    () => ({
      namespace: 'public-note-viewer',
      theme,
      nodes: editorNodes,
      editable: false,
      editorState: (editor: import('lexical').LexicalEditor) => {
        try {
          const content = JSON.stringify(note.content);
          const parsed = editor.parseEditorState(content);
          editor.setEditorState(parsed);
        } catch (e) {
          console.error('Failed to parse note content:', e);
        }
      },
      onError: (error: Error) => {
        console.error('Lexical error:', error);
      },
    }),
    [note.content]
  );

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="sticky top-0 z-50 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container mx-auto flex h-14 items-center justify-between px-4">
          <Link href="/" className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground">
            <ArrowLeft className="h-4 w-4" />
            <span>Back to Tradstry</span>
          </Link>
          <Badge variant="secondary" className="gap-1">
            <Eye className="h-3 w-3" />
            Public Note
          </Badge>
        </div>
      </header>

      {/* Content */}
      <main className="container mx-auto max-w-3xl px-4 py-8">
        {/* Title */}
        <h1 className="text-4xl font-bold tracking-tight mb-4">{note.title}</h1>

        {/* Meta info */}
        <div className="flex flex-wrap items-center gap-4 text-sm text-muted-foreground mb-6">
          <div className="flex items-center gap-1">
            <Calendar className="h-4 w-4" />
            <span>Updated {formatDate(note.updated_at)}</span>
          </div>
          <div className="flex items-center gap-1">
            <FileText className="h-4 w-4" />
            <span>{note.word_count} words</span>
          </div>
          {note.view_count !== undefined && (
            <div className="flex items-center gap-1">
              <Eye className="h-4 w-4" />
              <span>{note.view_count} views</span>
            </div>
          )}
        </div>

        <Separator className="mb-8" />

        {/* Note content */}
        <article className="prose prose-neutral dark:prose-invert max-w-none">
          <LexicalComposer initialConfig={initialConfig}>
            <RichTextPlugin
              contentEditable={
                <ContentEditable className="outline-none min-h-[200px]" />
              }
              placeholder={null}
              ErrorBoundary={LexicalErrorBoundary}
            />
          </LexicalComposer>
        </article>

        {/* Footer */}
        <Separator className="my-8" />
        <footer className="text-center text-sm text-muted-foreground">
          <p>
            Shared via{' '}
            <Link href="/" className="text-primary hover:underline">
              Tradstry
            </Link>
            {' '}— Your Trading Journal & Analytics Platform
          </p>
        </footer>
      </main>
    </div>
  );
}
