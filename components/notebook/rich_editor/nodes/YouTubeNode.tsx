'use client';

import type {
  DOMConversionMap,
  DOMConversionOutput,
  DOMExportOutput,
  EditorConfig,
  LexicalNode,
  NodeKey,
  SerializedLexicalNode,
  Spread,
} from 'lexical';
import { $applyNodeReplacement, DecoratorNode } from 'lexical';
import React, { Suspense } from 'react';

const YouTubeComponent = React.lazy(() => import('./YouTubeComponent'));

export interface YouTubePayload {
  videoId: string;
  key?: NodeKey;
}

export type SerializedYouTubeNode = Spread<
  {
    videoId: string;
  },
  SerializedLexicalNode
>;

function extractYouTubeVideoId(url: string): string | null {
  // Handle various YouTube URL formats
  const patterns = [
    /(?:youtube\.com\/watch\?v=|youtu\.be\/|youtube\.com\/embed\/)([a-zA-Z0-9_-]{11})/,
    /^([a-zA-Z0-9_-]{11})$/, // Direct video ID
  ];

  for (const pattern of patterns) {
    const match = url.match(pattern);
    if (match) {
      return match[1];
    }
  }
  return null;
}

function convertYouTubeElement(domNode: Node): null | DOMConversionOutput {
  if (domNode instanceof HTMLIFrameElement) {
    const src = domNode.getAttribute('src');
    if (src && src.includes('youtube.com/embed/')) {
      const videoId = extractYouTubeVideoId(src);
      if (videoId) {
        const node = $createYouTubeNode({ videoId });
        return { node };
      }
    }
  }
  return null;
}

export class YouTubeNode extends DecoratorNode<React.ReactNode> {
  __videoId: string;

  static getType(): string {
    return 'youtube';
  }

  static clone(node: YouTubeNode): YouTubeNode {
    return new YouTubeNode(node.__videoId, node.__key);
  }

  static importJSON(serializedNode: SerializedYouTubeNode): YouTubeNode {
    const { videoId } = serializedNode;
    return $createYouTubeNode({ videoId });
  }

  static importDOM(): DOMConversionMap | null {
    return {
      iframe: () => ({
        conversion: convertYouTubeElement,
        priority: 1,
      }),
    };
  }

  constructor(videoId: string, key?: NodeKey) {
    super(key);
    this.__videoId = videoId;
  }

  exportDOM(): DOMExportOutput {
    const element = document.createElement('iframe');
    element.setAttribute('src', `https://www.youtube.com/embed/${this.__videoId}`);
    element.setAttribute('width', '560');
    element.setAttribute('height', '315');
    element.setAttribute('frameborder', '0');
    element.setAttribute('allowfullscreen', 'true');
    element.setAttribute(
      'allow',
      'accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture'
    );
    return { element };
  }

  exportJSON(): SerializedYouTubeNode {
    return {
      videoId: this.__videoId,
      type: 'youtube',
      version: 1,
    };
  }

  getVideoId(): string {
    return this.__videoId;
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
      <Suspense fallback={<div className="h-[315px] w-full max-w-[560px] animate-pulse rounded-lg bg-muted" />}>
        <YouTubeComponent videoId={this.__videoId} nodeKey={this.getKey()} />
      </Suspense>
    );
  }
}

export function $createYouTubeNode({ videoId, key }: YouTubePayload): YouTubeNode {
  return $applyNodeReplacement(new YouTubeNode(videoId, key));
}

export function $isYouTubeNode(node: LexicalNode | null | undefined): node is YouTubeNode {
  return node instanceof YouTubeNode;
}

export { extractYouTubeVideoId };
