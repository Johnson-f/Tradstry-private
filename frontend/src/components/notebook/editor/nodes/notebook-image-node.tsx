"use client";

import {
  DecoratorNode,
  type LexicalNode,
  type NodeKey,
  type SerializedLexicalNode,
  type Spread,
} from "lexical";
import type { JSX } from "react";

export type SerializedNotebookImageNode = Spread<
  {
    type: "notebook-image";
    version: 1;
    imageId: string;
    src: string;
    altText: string;
    width: number;
    height: number;
  },
  SerializedLexicalNode
>;

function NotebookImageComponent({
  src,
  altText,
  width,
  height,
}: {
  src: string;
  altText: string;
  width: number;
  height: number;
}) {
  const isTemp = src?.startsWith("blob:") ?? false;

  return (
    <figure className="my-4 overflow-hidden rounded-2xl border border-slate-200 bg-slate-50">
      <img
        src={src ?? ""}
        alt={altText}
        width={width || undefined}
        height={height || undefined}
        loading={isTemp ? "eager" : "lazy"}
        className="h-auto max-h-[32rem] w-full object-contain"
      />
      {altText ? (
        <figcaption className="border-t border-slate-200 px-3 py-2 text-xs text-slate-500">
          {altText}
        </figcaption>
      ) : null}
    </figure>
  );
}

export class NotebookImageNode extends DecoratorNode<JSX.Element> {
  __imageId: string;
  __src: string;
  __altText: string;
  __width: number;
  __height: number;

  static getType(): string {
    return "notebook-image";
  }

  static clone(node: NotebookImageNode): NotebookImageNode {
    return new NotebookImageNode(
      node.__imageId,
      node.__src,
      node.__altText,
      node.__width,
      node.__height,
      node.__key,
    );
  }

  static importJSON(
    serializedNode: SerializedNotebookImageNode,
  ): NotebookImageNode {
    return $createNotebookImageNode({
      imageId: serializedNode.imageId,
      src: serializedNode.src,
      altText: serializedNode.altText,
      width: serializedNode.width,
      height: serializedNode.height,
    });
  }

  constructor(
    imageId: string,
    src: string,
    altText: string,
    width: number,
    height: number,
    key?: NodeKey,
  ) {
    super(key);
    this.__imageId = imageId;
    this.__src = src;
    this.__altText = altText;
    this.__width = width;
    this.__height = height;
  }

  exportJSON(): SerializedNotebookImageNode {
    return {
      ...super.exportJSON(),
      type: "notebook-image",
      version: 1,
      imageId: this.__imageId,
      src: this.__src,
      altText: this.__altText,
      width: this.__width,
      height: this.__height,
    };
  }

  createDOM(): HTMLElement {
    return document.createElement("div");
  }

  updateDOM(): false {
    return false;
  }

  isInline(): false {
    return false;
  }

  decorate(): JSX.Element {
    return (
      <NotebookImageComponent
        src={this.__src}
        altText={this.__altText}
        width={this.__width}
        height={this.__height}
      />
    );
  }
}

export function $createNotebookImageNode({
  imageId,
  src,
  altText = "",
  width = 0,
  height = 0,
}: {
  imageId: string;
  src: string;
  altText?: string;
  width?: number;
  height?: number;
}): NotebookImageNode {
  return new NotebookImageNode(imageId, src, altText, width, height);
}

export function $isNotebookImageNode(
  node: LexicalNode | null | undefined,
): node is NotebookImageNode {
  return node instanceof NotebookImageNode;
}
