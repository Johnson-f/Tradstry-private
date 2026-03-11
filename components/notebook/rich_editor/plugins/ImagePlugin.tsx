'use client';

import { useEffect, useCallback, useRef } from 'react';
import { useLexicalComposerContext } from '@lexical/react/LexicalComposerContext';
import {
  $insertNodes,
  COMMAND_PRIORITY_EDITOR,
  COMMAND_PRIORITY_HIGH,
  createCommand,
  LexicalCommand,
  PASTE_COMMAND,
  DROP_COMMAND,
} from 'lexical';
import { $createImageNode, ImagePayload } from '../nodes/ImageNode';
import { notebookImagesService } from '@/lib/services/notebook-service';
import { toast } from 'sonner';

export const INSERT_IMAGE_COMMAND: LexicalCommand<ImagePayload> =
  createCommand('INSERT_IMAGE_COMMAND');

// Command to upload and insert image (used when noteId is available)
export const UPLOAD_IMAGE_COMMAND: LexicalCommand<{ file: File; noteId: string }> =
  createCommand('UPLOAD_IMAGE_COMMAND');

interface ImagePluginProps {
  noteId?: string;
}

function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}

function isImageFile(file: File): boolean {
  return file.type.startsWith('image/');
}

const MAX_IMAGE_WIDTH = 2000;
const MAX_IMAGE_HEIGHT = 1200;

function getScaledDimensions(
  naturalWidth: number,
  naturalHeight: number,
): { width: number; height: number } {
  const aspectRatio = naturalWidth / naturalHeight;

  let width = naturalWidth;
  let height = naturalHeight;

  if (width > MAX_IMAGE_WIDTH) {
    width = MAX_IMAGE_WIDTH;
    height = width / aspectRatio;
  }

  if (height > MAX_IMAGE_HEIGHT) {
    height = MAX_IMAGE_HEIGHT;
    width = height * aspectRatio;
  }

  return { width: Math.round(width), height: Math.round(height) };
}

function loadImageDimensions(src: string): Promise<{ width: number; height: number }> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      const { width, height } = getScaledDimensions(img.naturalWidth, img.naturalHeight);
      resolve({ width, height });
    };
    img.onerror = reject;
    img.src = src;
  });
}

export function ImagePlugin({ noteId }: ImagePluginProps): null {
  const [editor] = useLexicalComposerContext();
  const noteIdRef = useRef(noteId);

  // Keep ref updated
  useEffect(() => {
    noteIdRef.current = noteId;
  }, [noteId]);

  // Upload image to backend and return the image data
  const uploadImage = useCallback(async (
    file: File,
    currentNoteId: string,
    dimensions: { width: number; height: number }
  ): Promise<{ src: string; imageId: string } | null> => {
    try {
      const response = await notebookImagesService.uploadFile(currentNoteId, file, {
        altText: file.name || 'Uploaded image',
        width: dimensions.width,
        height: dimensions.height,
      });

      if (response.success && response.data) {
        return {
          src: response.data.src,
          imageId: response.data.id,
        };
      }
      console.error('Image upload failed:', response.message);
      return null;
    } catch (error) {
      console.error('Failed to upload image:', error);
      return null;
    }
  }, []);

  // Process image file - upload if noteId exists, otherwise use base64
  const processImageFile = useCallback(async (
    file: File,
    altText: string = 'Image'
  ) => {
    try {
      const base64 = await fileToBase64(file);
      const { width, height } = await loadImageDimensions(base64);
      const currentNoteId = noteIdRef.current;

      // If we have a noteId, upload to backend
      if (currentNoteId) {
        // Show uploading toast with spinner
        const toastId = toast.loading('Uploading image...', {
          description: file.name || 'Processing your image',
        });

        const uploadResult = await uploadImage(file, currentNoteId, { width, height });
        
        if (uploadResult) {
          // Update toast to success
          toast.success('Image uploaded successfully', {
            id: toastId,
            description: file.name || 'Your image is ready',
          });

          // Use the uploaded URL and imageId
          editor.dispatchCommand(INSERT_IMAGE_COMMAND, {
            src: uploadResult.src,
            altText: file.name || altText,
            width,
            height,
            imageId: uploadResult.imageId,
          });
          return;
        }
        
        // Upload failed - show error and fall through to base64
        toast.error('Upload failed', {
          id: toastId,
          description: 'Using local image instead',
        });
        console.warn('Upload failed, falling back to base64');
      }

      // No noteId or upload failed - use base64 (will be uploaded when note is saved)
      editor.dispatchCommand(INSERT_IMAGE_COMMAND, {
        src: base64,
        altText: file.name || altText,
        width,
        height,
      });
    } catch (error) {
      console.error('Failed to process image:', error);
      toast.error('Failed to process image', {
        description: 'Please try again',
      });
    }
  }, [editor, uploadImage]);

  useEffect(() => {
    // Register INSERT_IMAGE_COMMAND
    const unregisterInsertImage = editor.registerCommand(
      INSERT_IMAGE_COMMAND,
      (payload: ImagePayload) => {
        const imageNode = $createImageNode(payload);
        $insertNodes([imageNode]);
        return true;
      },
      COMMAND_PRIORITY_EDITOR,
    );

    // Register UPLOAD_IMAGE_COMMAND for explicit uploads
    const unregisterUploadImage = editor.registerCommand(
      UPLOAD_IMAGE_COMMAND,
      ({ file }) => {
        processImageFile(file, file.name || 'Uploaded image');
        return true;
      },
      COMMAND_PRIORITY_EDITOR,
    );

    // Handle paste events
    const unregisterPaste = editor.registerCommand(
      PASTE_COMMAND,
      (event: ClipboardEvent) => {
        const clipboardData = event.clipboardData;
        if (!clipboardData) return false;

        const files = Array.from(clipboardData.files);
        const imageFiles = files.filter(isImageFile);

        if (imageFiles.length === 0) return false;

        event.preventDefault();

        imageFiles.forEach((file) => {
          processImageFile(file, 'Pasted image');
        });

        return true;
      },
      COMMAND_PRIORITY_HIGH,
    );

    // Handle drop events
    const unregisterDrop = editor.registerCommand(
      DROP_COMMAND,
      (event: DragEvent) => {
        const dataTransfer = event.dataTransfer;
        if (!dataTransfer) return false;

        const files = Array.from(dataTransfer.files);
        const imageFiles = files.filter(isImageFile);

        if (imageFiles.length === 0) return false;

        event.preventDefault();

        imageFiles.forEach((file) => {
          processImageFile(file, 'Dropped image');
        });

        return true;
      },
      COMMAND_PRIORITY_HIGH,
    );

    return () => {
      unregisterInsertImage();
      unregisterUploadImage();
      unregisterPaste();
      unregisterDrop();
    };
  }, [editor, processImageFile]);

  return null;
}

export default ImagePlugin;
