"use client";

import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useMemo } from "react";
import type { Components } from "react-markdown";
import { cn } from "@/lib/utils";

interface MarkdownFormatterProps {
  content: string;
  className?: string;
}

const getMarkdownComponents = (): Components => ({
  h1: ({ ...props }) => (
    <h1
      className="text-xl font-bold mt-4 mb-3 text-gray-900 dark:text-white first:mt-0"
      {...props}
    />
  ),
  h2: ({ ...props }) => (
    <h2
      className="text-lg font-bold mt-4 mb-2 text-gray-900 dark:text-white first:mt-0"
      {...props}
    />
  ),
  h3: ({ ...props }) => (
    <h3
      className="text-base font-bold mt-3 mb-2 text-gray-900 dark:text-white first:mt-0"
      {...props}
    />
  ),
  p: ({ ...props }) => (
    <p
      className="mb-3 leading-relaxed text-gray-800 dark:text-gray-100 last:mb-0"
      {...props}
    />
  ),
  ul: ({ ...props }) => (
    <ul
      className="list-disc ml-6 mb-3 space-y-1 text-gray-800 dark:text-gray-100"
      {...props}
    />
  ),
  ol: ({ ...props }) => (
    <ol
      className="list-decimal ml-6 mb-3 space-y-1 text-gray-800 dark:text-gray-100"
      {...props}
    />
  ),
  li: ({ ...props }) => (
    <li className="text-gray-800 dark:text-gray-100" {...props} />
  ),
  code: ({ className, children, ...props }) => {
    const isInline = !className?.includes("language-");
    return isInline ? (
      <code
        className="bg-gray-200 dark:bg-gray-800 text-orange-600 dark:text-orange-400 px-1.5 py-0.5 rounded text-sm font-mono"
        {...props}
      >
        {children}
      </code>
    ) : (
      <code
        className="block bg-gray-100 dark:bg-gray-900 text-green-600 dark:text-green-400 p-4 rounded-lg my-3 overflow-x-auto text-sm font-mono"
        {...props}
      >
        {children}
      </code>
    );
  },
  a: ({ ...props }) => (
    <a
      className="text-blue-500 dark:text-blue-300 underline"
      target="_blank"
      rel="noopener noreferrer"
      {...props}
    />
  ),
});

export function MarkdownFormatter({ content, className }: MarkdownFormatterProps) {
  const markdownComponents = useMemo(() => getMarkdownComponents(), []);

  return (
    <div className={cn("prose prose-sm dark:prose-invert max-w-none", className)}>
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
        {content}
      </ReactMarkdown>
    </div>
  );
}

