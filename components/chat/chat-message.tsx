"use client";

import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { cn } from "@/lib/utils";
import { memo, useMemo } from 'react';
import type { Components } from 'react-markdown';

interface Message {
  id: string;
  role: "user" | "assistant" | string;
  content: string;
  created_at: string;
  message_type?: string;
}

// Shared markdown components configuration for consistent rendering
const getMarkdownComponents = (): Components => ({
  h1: (props) => (
    <h1 className="text-xl font-bold mt-4 mb-3 text-gray-900 dark:text-white first:mt-0" {...props} />
  ),
  h2: (props) => (
    <h2 className="text-lg font-bold mt-4 mb-2 text-gray-900 dark:text-white first:mt-0" {...props} />
  ),
  h3: (props) => (
    <h3 className="text-base font-bold mt-3 mb-2 text-gray-900 dark:text-white first:mt-0" {...props} />
  ),
  h4: (props) => (
    <h4 className="text-sm font-semibold mt-2 mb-1 text-gray-900 dark:text-white first:mt-0" {...props} />
  ),
  p: (props) => (
    <p className="mb-3 leading-relaxed text-gray-800 dark:text-gray-100 last:mb-0" {...props} />
  ),
  ul: (props) => (
    <ul className="list-disc ml-6 mb-3 space-y-1 text-gray-800 dark:text-gray-100" {...props} />
  ),
  ol: (props) => (
    <ol className="list-decimal ml-6 mb-3 space-y-1 text-gray-800 dark:text-gray-100" {...props} />
  ),
  li: (props) => (
    <li className="text-gray-800 dark:text-gray-100" {...props} />
  ),
  table: (props) => (
    <div className="overflow-x-auto my-4 rounded-lg border border-gray-300 dark:border-gray-600">
      <table className="min-w-full divide-y divide-gray-300 dark:divide-gray-600" {...props} />
    </div>
  ),
  thead: (props) => (
    <thead className="bg-gray-100 dark:bg-gray-800" {...props} />
  ),
  tbody: (props) => (
    <tbody className="divide-y divide-gray-300 dark:divide-gray-600 bg-white dark:bg-gray-750" {...props} />
  ),
  th: (props) => (
    <th className="px-4 py-3 text-left text-xs font-semibold text-gray-700 dark:text-gray-200 uppercase tracking-wider" {...props} />
  ),
  td: (props) => (
    <td className="px-4 py-3 text-sm text-gray-800 dark:text-gray-100 whitespace-nowrap" {...props} />
  ),
  strong: (props) => (
    <strong className="font-bold text-blue-600 dark:text-blue-400" {...props} />
  ),
  em: (props) => (
    <em className="italic text-gray-700 dark:text-gray-300" {...props} />
  ),
  code: ({ className, children, ...props }) => {
    const isInline = !className?.includes('language-');
    return isInline ? (
      <code className="bg-gray-200 dark:bg-gray-800 text-orange-600 dark:text-orange-400 px-1.5 py-0.5 rounded text-sm font-mono" {...props}>
        {children}
      </code>
    ) : (
      <code className="block bg-gray-100 dark:bg-gray-900 text-green-600 dark:text-green-400 p-4 rounded-lg my-3 overflow-x-auto text-sm font-mono" {...props}>
        {children}
      </code>
    );
  },
  pre: (props) => (
    <pre className="my-3" {...props} />
  ),
  a: (props) => (
    <a className="text-blue-400 hover:text-blue-300 underline" target="_blank" rel="noopener noreferrer" {...props} />
  ),
  hr: (props) => (
    <hr className="my-4 border-gray-600" {...props} />
  ),
  blockquote: (props) => (
    <blockquote className="border-l-4 border-blue-500 pl-4 py-2 my-3 italic text-gray-300 bg-gray-800 rounded" {...props} />
  ),
});

const formatMessageTime = (timestamp: string): string => {
  try {
    const date = new Date(timestamp);
    if (isNaN(date.getTime())) {
      return "Invalid Date";
    }
    return date.toLocaleTimeString(undefined, {
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch (error) {
    console.error("Error formatting timestamp:", timestamp, error);
    return "Invalid Date";
  }
};

// Memoized to prevent unnecessary re-renders when parent updates
export const ChatMessage = memo(({ message }: { message: Message }) => {
  const isUser = message.role === "user" || message.message_type === "user_question";
  const markdownComponents = useMemo(() => getMarkdownComponents(), []);

  return (
    <div className={cn("mb-6 flex", isUser ? "justify-end" : "justify-start")}>
      <div className="max-w-[58%] space-y-1">
        {isUser ? (
          <>
            <p className="text-sm leading-relaxed whitespace-pre-wrap text-gray-800 dark:text-gray-200">
              {message.content}
            </p>
            <p className="text-xs text-gray-500 dark:text-gray-500 text-right">
              {formatMessageTime(message.created_at)}
            </p>
          </>
        ) : (
          <>
            <div className="prose prose-sm dark:prose-invert prose-slate max-w-none">
              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                components={markdownComponents}
              >
                {message.content}
              </ReactMarkdown>
            </div>
            <p className="text-xs text-gray-500 text-left">
              {formatMessageTime(message.created_at)}
            </p>
          </>
        )}
      </div>
    </div>
  );
});

export const AIThinkingIndicator = () => (
  <div className="mb-6 flex justify-start">
    <div className="flex items-center gap-3 px-4 py-3 rounded-lg bg-gray-800/50 border border-gray-700">
        <div className="relative w-8 h-8 flex items-center justify-center">
          <div className="absolute inset-0 rounded-full bg-gradient-to-r from-orange-500 to-purple-500 animate-spin" style={{ animationDuration: '2s' }}>
            <div className="absolute inset-1 rounded-full bg-gray-900"></div>
          </div>
          <svg className="w-5 h-5 text-orange-400 relative z-10" fill="currentColor" viewBox="0 0 20 20">
            <path d="M10 2a8 8 0 100 16 8 8 0 000-16zM9 9a1 1 0 012 0v4a1 1 0 11-2 0V9zm1-4a1 1 0 100 2 1 1 0 000-2z"/>
          </svg>
        </div>
        <div className="flex flex-col">
          <span className="text-sm font-medium text-gray-200">AI is thinking...</span>
          <div className="flex items-center gap-1 mt-1">
            <div className="w-1.5 h-1.5 bg-orange-500 rounded-full animate-bounce"></div>
            <div className="w-1.5 h-1.5 bg-orange-500 rounded-full animate-bounce" style={{ animationDelay: '0.15s' }}></div>
            <div className="w-1.5 h-1.5 bg-orange-500 rounded-full animate-bounce" style={{ animationDelay: '0.3s' }}></div>
          </div>
        </div>
      </div>
    </div>
);

export const StreamingMessage = memo(({ content }: { content: string }) => {
  const markdownComponents = useMemo(() => getMarkdownComponents(), []);

  return (
    <div className="mb-6 flex justify-start">
      <div className="max-w-[58%] space-y-1">
        <div className="prose prose-sm dark:prose-invert prose-slate max-w-none">
          <ReactMarkdown
            remarkPlugins={[remarkGfm]}
            components={markdownComponents}
          >
            {content}
          </ReactMarkdown>
          <span className="inline-block w-2 h-5 bg-orange-500 ml-1 animate-pulse" />
        </div>
        <div className="flex items-center justify-start gap-2 text-xs text-gray-500">
          <div className="flex space-x-1">
            <div className="w-1 h-1 bg-orange-500 rounded-full animate-bounce" />
            <div className="w-1 h-1 bg-orange-500 rounded-full animate-bounce" style={{ animationDelay: '0.1s' }} />
            <div className="w-1 h-1 bg-orange-500 rounded-full animate-bounce" style={{ animationDelay: '0.2s' }} />
          </div>
          <span>Streaming...</span>
        </div>
      </div>
    </div>
  );
});

// Add display names for better debugging
ChatMessage.displayName = 'ChatMessage';
StreamingMessage.displayName = 'StreamingMessage';
AIThinkingIndicator.displayName = 'AIThinkingIndicator';
