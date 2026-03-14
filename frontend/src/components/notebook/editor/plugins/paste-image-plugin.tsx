"use client";

import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import { $getNodeByKey, $insertNodes } from "lexical";
import { useEffect } from "react";
import type { NotebookImage } from "@/lib/types/notebook";
import {
  $createNotebookImageNode,
  $isNotebookImageNode,
} from "../nodes/notebook-image-node";

function createTempImageId() {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return `local-${crypto.randomUUID()}`;
  }
  return `local-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

export function PasteImagePlugin({
  onUploadImage,
}: {
  onUploadImage?: (file: File) => Promise<NotebookImage>;
}) {
  const [editor] = useLexicalComposerContext();

  useEffect(() => {
    if (!onUploadImage) {
      return;
    }

    return editor.registerRootListener((rootElement, previousRootElement) => {
      if (previousRootElement) {
        previousRootElement.onpaste = null;
      }

      if (!rootElement) {
        return;
      }

      rootElement.onpaste = (event) => {
        const imageFiles = new Map<string, File>();
        const clipboardFiles = Array.from(event.clipboardData?.files ?? []);
        const clipboardItems = Array.from(
          event.clipboardData?.items ?? [],
        ).filter(
          (item) => item.kind === "file" && item.type.startsWith("image/"),
        );

        for (const file of clipboardFiles) {
          if (file.type.startsWith("image/")) {
            imageFiles.set(`${file.name}:${file.size}:${file.type}`, file);
          }
        }

        for (const item of clipboardItems) {
          const file = item.getAsFile();
          if (!file || !file.type.startsWith("image/")) continue;
          imageFiles.set(`${file.name}:${file.size}:${file.type}`, file);
        }

        const files = Array.from(imageFiles.values());
        if (files.length === 0) return;

        event.preventDefault();

        const pending = files.map((file) => ({
          file,
          localSrc: URL.createObjectURL(file),
          tempId: createTempImageId(),
        }));

        const nodeKeys: string[] = [];

        editor.update(() => {
          const nodes = pending.map(({ localSrc, tempId, file }) => {
            const node = $createNotebookImageNode({
              imageId: tempId,
              src: localSrc,
              altText: file.name,
            });
            nodeKeys.push(node.getKey());
            return node;
          });
          $insertNodes(nodes);
        });

        void Promise.all(
          pending.map(async ({ file, localSrc }, i) => {
            try {
              const uploaded: NotebookImage = await onUploadImage(file);

              // Update node properties in place instead of replacing —
              // replacing changes the key and Lexical loses track of the node
              editor.update(() => {
                const liveNode = $getNodeByKey(nodeKeys[i]!);
                if (!liveNode || !$isNotebookImageNode(liveNode)) return;

                const writable = liveNode.getWritable();
                writable.__imageId = uploaded.id;
                writable.__src = uploaded.secureUrl;
                writable.__altText = uploaded.originalFilename || file.name;
                writable.__width = uploaded.width ?? 0;
                writable.__height = uploaded.height ?? 0;
              });

              URL.revokeObjectURL(localSrc);
            } catch (error) {
              URL.revokeObjectURL(localSrc);

              editor.update(() => {
                const liveNode = $getNodeByKey(nodeKeys[i]!);
                if (!liveNode || !$isNotebookImageNode(liveNode)) return;
                liveNode.remove();
              });

              console.error("Failed to upload pasted notebook image", error);
              window.alert("Failed to upload pasted image.");
            }
          }),
        );
      };
    });
  }, [editor, onUploadImage]);

  return null;
}
