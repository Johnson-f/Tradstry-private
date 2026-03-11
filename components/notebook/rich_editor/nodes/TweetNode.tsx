'use client';

import type {
  DOMConversionMap,
  DOMExportOutput,
  EditorConfig,
  LexicalNode,
  NodeKey,
  SerializedLexicalNode,
  Spread,
} from 'lexical';
import { $applyNodeReplacement, DecoratorNode } from 'lexical';
import React, { Suspense } from 'react';

const TweetComponent = React.lazy(() => import('./TweetComponent'));

export interface TweetPayload {
  tweetId: string;
  key?: NodeKey;
}

export type SerializedTweetNode = Spread<
  {
    tweetId: string;
  },
  SerializedLexicalNode
>;

function extractTweetId(url: string): string | null {
  // Handle various Twitter/X URL formats
  const patterns = [
    /(?:twitter\.com|x\.com)\/\w+\/status\/(\d+)/,
    /^(\d{10,})$/, // Direct tweet ID (10+ digits)
  ];

  for (const pattern of patterns) {
    const match = url.match(pattern);
    if (match) {
      return match[1];
    }
  }
  return null;
}

export class TweetNode extends DecoratorNode<React.ReactNode> {
  __tweetId: string;

  static getType(): string {
    return 'tweet';
  }

  static clone(node: TweetNode): TweetNode {
    return new TweetNode(node.__tweetId, node.__key);
  }

  static importJSON(serializedNode: SerializedTweetNode): TweetNode {
    const { tweetId } = serializedNode;
    return $createTweetNode({ tweetId });
  }

  static importDOM(): DOMConversionMap | null {
    return null; // Twitter embeds are complex, skip DOM import
  }

  constructor(tweetId: string, key?: NodeKey) {
    super(key);
    this.__tweetId = tweetId;
  }

  exportDOM(): DOMExportOutput {
    const element = document.createElement('blockquote');
    element.setAttribute('class', 'twitter-tweet');
    element.setAttribute('data-tweet-id', this.__tweetId);
    
    const link = document.createElement('a');
    link.setAttribute('href', `https://twitter.com/i/status/${this.__tweetId}`);
    link.textContent = 'View Tweet';
    element.appendChild(link);
    
    return { element };
  }

  exportJSON(): SerializedTweetNode {
    return {
      tweetId: this.__tweetId,
      type: 'tweet',
      version: 1,
    };
  }

  getTweetId(): string {
    return this.__tweetId;
  }

  createDOM(_config: EditorConfig): HTMLElement {
    const div = document.createElement('div');
    div.style.display = 'flex';
    div.style.justifyContent = 'center';
    div.style.width = '100%';
    div.style.margin = '1rem 0';
    return div;
  }

  updateDOM(): false {
    return false;
  }

  decorate(): React.ReactNode {
    return (
      <Suspense fallback={<div className="mx-auto h-[200px] w-full max-w-[550px] animate-pulse rounded-lg bg-muted" />}>
        <TweetComponent tweetId={this.__tweetId} nodeKey={this.getKey()} />
      </Suspense>
    );
  }
}

export function $createTweetNode({ tweetId, key }: TweetPayload): TweetNode {
  return $applyNodeReplacement(new TweetNode(tweetId, key));
}

export function $isTweetNode(node: LexicalNode | null | undefined): node is TweetNode {
  return node instanceof TweetNode;
}

export { extractTweetId };
