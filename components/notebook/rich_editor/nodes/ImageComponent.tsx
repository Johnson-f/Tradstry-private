'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
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
  SELECTION_CHANGE_COMMAND,
} from 'lexical';
import { cn } from '@/lib/utils';
import { $isImageNode } from './ImageNode';
import { MessageSquare, AlignCenter, Copy, ZoomIn, Download, MoreHorizontal, Trash2, X } from 'lucide-react';
import { toast } from 'sonner';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';

interface ImageComponentProps {
  src: string;
  altText: string;
  width?: number;
  height?: number;
  nodeKey: string;
  imageId?: string;
  caption?: string;
}

export default function ImageComponent({
  src,
  altText,
  width,
  height,
  nodeKey,
  imageId,
  caption,
}: ImageComponentProps) {
  const imageRef = useRef<HTMLImageElement>(null);
  const [editor] = useLexicalComposerContext();
  const [isSelected, setSelected, clearSelection] = useLexicalNodeSelection(nodeKey);
  const [isResizing, setIsResizing] = useState(false);
  const [currentWidth, setCurrentWidth] = useState(width);
  const [currentHeight, setCurrentHeight] = useState(height);
  const [naturalWidth, setNaturalWidth] = useState(0);
  const [naturalHeight, setNaturalHeight] = useState(0);
  const [customWidth, setCustomWidth] = useState('');
  const [isPopoverOpen, setIsPopoverOpen] = useState(false);
  const [isZoomOpen, setIsZoomOpen] = useState(false);
  const [currentCaption, setCurrentCaption] = useState(caption || '');
  const [isEditingCaption, setIsEditingCaption] = useState(false);
  const captionInputRef = useRef<HTMLInputElement>(null);

  // Get natural dimensions when image loads
  useEffect(() => {
    if (imageRef.current?.complete) {
      setNaturalWidth(imageRef.current.naturalWidth);
      setNaturalHeight(imageRef.current.naturalHeight);
    }
  }, []);

  const onDelete = useCallback(
    (event: KeyboardEvent) => {
      if (isSelected && $isNodeSelection($getSelection())) {
        event.preventDefault();
        editor.update(() => {
          const node = $getNodeByKey(nodeKey);
          if ($isImageNode(node)) {
            node.remove();
          }
        });
      }
      return false;
    },
    [editor, isSelected, nodeKey],
  );

  useEffect(() => {
    return mergeRegister(
      editor.registerCommand(
        SELECTION_CHANGE_COMMAND,
        () => {
          return false;
        },
        COMMAND_PRIORITY_LOW,
      ),
      editor.registerCommand(
        CLICK_COMMAND,
        (event: MouseEvent) => {
          if (imageRef.current?.contains(event.target as Node)) {
            if (!event.shiftKey) {
              clearSelection();
            }
            setSelected(true);
            return true;
          }
          return false;
        },
        COMMAND_PRIORITY_LOW,
      ),
      editor.registerCommand(
        KEY_DELETE_COMMAND,
        onDelete,
        COMMAND_PRIORITY_LOW,
      ),
      editor.registerCommand(
        KEY_BACKSPACE_COMMAND,
        onDelete,
        COMMAND_PRIORITY_LOW,
      ),
    );
  }, [clearSelection, editor, onDelete, setSelected]);

  const handleResizeStart = useCallback(
    (e: React.MouseEvent, side: 'left' | 'right') => {
      e.preventDefault();
      e.stopPropagation();
      setIsResizing(true);

      const startX = e.clientX;
      const startWidth = currentWidth || imageRef.current?.naturalWidth || 300;
      const startHeight = currentHeight || imageRef.current?.naturalHeight || 200;
      const aspectRatio = startWidth / startHeight;

      const handleMouseMove = (moveEvent: MouseEvent) => {
        const deltaX = moveEvent.clientX - startX;

        let newWidth = startWidth;

        if (side === 'right') {
          newWidth = Math.max(50, startWidth + deltaX);
        } else if (side === 'left') {
          newWidth = Math.max(50, startWidth - deltaX);
        }

        // Maintain aspect ratio
        const newHeight = newWidth / aspectRatio;

        setCurrentWidth(Math.round(newWidth));
        setCurrentHeight(Math.round(newHeight));
      };

      const handleMouseUp = () => {
        setIsResizing(false);
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);

        // Update the node with new dimensions
        editor.update(() => {
          const node = $getNodeByKey(nodeKey);
          if ($isImageNode(node)) {
            node.setWidthAndHeight(
              currentWidth || startWidth,
              currentHeight || startHeight,
            );
          }
        });
      };

      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
    },
    [currentWidth, currentHeight, editor, nodeKey],
  );

  const handleDownload = async () => {
    try {
      const response = await fetch(src);
      const blob = await response.blob();
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = altText || 'image';
      link.click();
      URL.revokeObjectURL(url);
      toast.success('Image downloaded successfully');
    } catch (error) {
      console.error('Failed to download image:', error);
      toast.error('Failed to download image');
    }
  };

  const handleDelete = useCallback(async () => {
    // Delete from backend if we have an imageId
    if (imageId) {
      const toastId = toast.loading('Deleting image...', {
        description: 'Removing from storage',
      });

      try {
        const { notebookImagesService } = await import('@/lib/services/notebook-service');
        await notebookImagesService.delete(imageId);
        
        toast.success('Image deleted successfully', {
          id: toastId,
        });
      } catch (error) {
        console.error('Failed to delete image from backend:', error);
        toast.error('Failed to delete from storage', {
          id: toastId,
          description: 'Image removed from editor only',
        });
        // Continue to remove from editor even if backend delete fails
      }
    }

    // Remove from editor
    editor.update(() => {
      const node = $getNodeByKey(nodeKey);
      if ($isImageNode(node)) {
        node.remove();
      }
    });
  }, [editor, nodeKey, imageId]);

  const handleCopy = async () => {
    try {
      const response = await fetch(src);
      const blob = await response.blob();
      await navigator.clipboard.write([
        new ClipboardItem({ [blob.type]: blob })
      ]);
      toast.success('Image successfully copied');
    } catch (error) {
      console.error('Failed to copy image:', error);
      toast.error('Failed to copy image');
    }
  };

  const handleSizeChange = (value: string) => {
    const aspectRatio = naturalWidth / naturalHeight;
    let newWidth = currentWidth || naturalWidth;
    let newHeight = currentHeight || naturalHeight;

    switch (value) {
      case 'small':
        newWidth = 300;
        newHeight = Math.round(newWidth / aspectRatio);
        break;
      case 'medium':
        newWidth = 600;
        newHeight = Math.round(newWidth / aspectRatio);
        break;
      case 'large':
        newWidth = 1000;
        newHeight = Math.round(newWidth / aspectRatio);
        break;
      case 'original':
        newWidth = naturalWidth;
        newHeight = naturalHeight;
        break;
    }

    setCurrentWidth(newWidth);
    setCurrentHeight(newHeight);

    editor.update(() => {
      const node = $getNodeByKey(nodeKey);
      if ($isImageNode(node)) {
        node.setWidthAndHeight(newWidth, newHeight);
      }
    });
  };

  const handleCustomWidthApply = () => {
    const newWidth = parseInt(customWidth);
    if (!isNaN(newWidth) && newWidth > 0) {
      const aspectRatio = naturalWidth / naturalHeight;
      const newHeight = Math.round(newWidth / aspectRatio);

      setCurrentWidth(newWidth);
      setCurrentHeight(newHeight);

      editor.update(() => {
        const node = $getNodeByKey(nodeKey);
        if ($isImageNode(node)) {
          node.setWidthAndHeight(newWidth, newHeight);
        }
      });

      setCustomWidth('');
      setIsPopoverOpen(false);
    }
  };

  const handleCaptionSave = async () => {
    const trimmedCaption = currentCaption.trim();
    
    // Update the node
    editor.update(() => {
      const node = $getNodeByKey(nodeKey);
      if ($isImageNode(node)) {
        node.setCaption(trimmedCaption);
      }
    });

    // Save to database if we have an imageId
    if (imageId) {
      try {
        const { notebookImagesService } = await import('@/lib/services/notebook-service');
        await notebookImagesService.update(imageId, { caption: trimmedCaption || undefined });
        toast.success('Caption saved');
      } catch (error) {
        console.error('Failed to save caption:', error);
        toast.error('Failed to save caption');
      }
    }

    setIsEditingCaption(false);
  };

  const handleCaptionKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleCaptionSave();
    } else if (e.key === 'Escape') {
      setCurrentCaption(caption || '');
      setIsEditingCaption(false);
    }
  };

  // Focus caption input when editing starts
  useEffect(() => {
    if (isEditingCaption && captionInputRef.current) {
      captionInputRef.current.focus();
    }
  }, [isEditingCaption]);

  const displayWidth = currentWidth || naturalWidth || 0;
  const displayHeight = currentHeight || naturalHeight || 0;

  return (
    <div className="relative inline-block">
      {/* Controls Toolbar - Shows when image is selected */}
      {isSelected && (
        <div className="absolute -top-12 right-0 z-10 bg-gray-900 text-white rounded-lg shadow-lg px-2 py-1.5 flex items-center gap-1">
          <button
            className="p-2 hover:bg-gray-700 rounded transition-colors"
            title="Ask AI"
            onClick={(e) => e.stopPropagation()}
          >
            <MessageSquare className="w-4 h-4" />
          </button>
          <button
            className="p-2 hover:bg-gray-700 rounded transition-colors"
            title="Center"
            onClick={(e) => e.stopPropagation()}
          >
            <AlignCenter className="w-4 h-4" />
          </button>
          <button
            className="p-2 hover:bg-gray-700 rounded transition-colors"
            title="Copy"
            onClick={(e) => {
              e.stopPropagation();
              handleCopy();
            }}
          >
            <Copy className="w-4 h-4" />
          </button>
          <button
            className="p-2 hover:bg-gray-700 rounded transition-colors"
            title="Zoom"
            onClick={(e) => {
              e.stopPropagation();
              setIsZoomOpen(true);
            }}
          >
            <ZoomIn className="w-4 h-4" />
          </button>
          <button
            className="p-2 hover:bg-gray-700 rounded transition-colors"
            title="Download"
            onClick={(e) => {
              e.stopPropagation();
              handleDownload();
            }}
          >
            <Download className="w-4 h-4" />
          </button>
          
          {/* Dimensions Popover */}
          <Popover open={isPopoverOpen} onOpenChange={setIsPopoverOpen}>
            <PopoverTrigger asChild>
              <button
                className="px-2 py-2 hover:bg-gray-700 rounded transition-colors text-xs font-mono"
                title="Resize options"
                onClick={(e) => e.stopPropagation()}
              >
                {displayWidth} × {displayHeight}
              </button>
            </PopoverTrigger>
            <PopoverContent className="w-80" onClick={(e) => e.stopPropagation()}>
              <div className="space-y-4">
                <div>
                  <h4 className="font-medium mb-2">Image Size</h4>
                  <p className="text-sm text-muted-foreground mb-3">
                    Current: {displayWidth} × {displayHeight}px
                  </p>
                  <p className="text-sm text-muted-foreground mb-3">
                    Original: {naturalWidth} × {naturalHeight}px
                  </p>
                </div>

                <div className="space-y-2">
                  <Label>Preset Sizes</Label>
                  <Select onValueChange={handleSizeChange}>
                    <SelectTrigger>
                      <SelectValue placeholder="Choose a size" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="small">Small (300px)</SelectItem>
                      <SelectItem value="medium">Medium (600px)</SelectItem>
                      <SelectItem value="large">Large (1000px)</SelectItem>
                      <SelectItem value="original">Original Size ({naturalWidth}px)</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="custom-width">Custom Width (px)</Label>
                  <div className="flex gap-2">
                    <Input
                      id="custom-width"
                      type="number"
                      placeholder={`e.g., ${displayWidth}`}
                      value={customWidth}
                      onChange={(e) => setCustomWidth(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') {
                          handleCustomWidthApply();
                        }
                      }}
                    />
                    <button
                      onClick={handleCustomWidthApply}
                      className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors"
                    >
                      Apply
                    </button>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    Height will adjust automatically to maintain aspect ratio
                  </p>
                </div>
              </div>
            </PopoverContent>
          </Popover>

          <button
            className="p-2 hover:bg-gray-700 rounded transition-colors"
            title="More"
            onClick={(e) => e.stopPropagation()}
          >
            <MoreHorizontal className="w-4 h-4" />
          </button>

          {/* Delete button */}
          <button
            className="p-2 hover:bg-red-600 rounded transition-colors"
            title="Delete image"
            onClick={(e) => {
              e.stopPropagation();
              handleDelete();
            }}
          >
            <Trash2 className="w-4 h-4" />
          </button>
        </div>
      )}

      <div
        className={cn(
          'relative inline-block',
          isSelected && 'ring-2 ring-primary ring-offset-2',
        )}
      >
        <img
          ref={imageRef}
          src={src}
          alt={altText}
          width={currentWidth}
          height={currentHeight}
          className={cn(
            'max-w-full rounded-md',
            isResizing && 'pointer-events-none',
          )}
          draggable={false}
          onLoad={(e) => {
            const img = e.currentTarget;
            setNaturalWidth(img.naturalWidth);
            setNaturalHeight(img.naturalHeight);
          }}
        />
        {isSelected && (
          <>
            {/* Left resize handle */}
            <div
              className="absolute top-1/2 -left-1 w-2 h-8 -translate-y-1/2 bg-primary rounded cursor-ew-resize"
              onMouseDown={(e) => handleResizeStart(e, 'left')}
            />
            {/* Right resize handle */}
            <div
              className="absolute top-1/2 -right-1 w-2 h-8 -translate-y-1/2 bg-primary rounded cursor-ew-resize"
              onMouseDown={(e) => handleResizeStart(e, 'right')}
            />
          </>
        )}
      </div>

      {/* Caption */}
      <div className="mt-2 text-center">
        {isEditingCaption ? (
          <Input
            ref={captionInputRef}
            value={currentCaption}
            onChange={(e) => setCurrentCaption(e.target.value)}
            onBlur={handleCaptionSave}
            onKeyDown={handleCaptionKeyDown}
            placeholder="Add a caption..."
            className="text-sm text-center text-muted-foreground max-w-md mx-auto"
          />
        ) : (
          <button
            onClick={() => setIsEditingCaption(true)}
            className={cn(
              "text-sm italic cursor-pointer hover:text-foreground transition-colors px-2 py-1 rounded",
              currentCaption ? "text-muted-foreground" : "text-muted-foreground/50"
            )}
          >
            {currentCaption || "Add a caption..."}
          </button>
        )}
      </div>

      {/* Zoom Modal */}
      {isZoomOpen && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm"
          onClick={() => setIsZoomOpen(false)}
        >
          <button
            className="absolute top-4 right-4 p-2 text-white hover:bg-white/20 rounded-full transition-colors"
            onClick={() => setIsZoomOpen(false)}
            title="Close"
          >
            <X className="w-6 h-6" />
          </button>
          <img
            src={src}
            alt={altText}
            className="max-w-[90vw] max-h-[90vh] object-contain rounded-lg"
            onClick={(e) => e.stopPropagation()}
          />
        </div>
      )}
    </div>
  );
}
