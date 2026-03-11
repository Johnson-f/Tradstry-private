// 'use client';

// import React, { useMemo } from 'react';
// import { cn } from '@/lib/utils';
// import BlockEditor from '@/components/notebook/BlockEditor';
// import { useTradeNote } from '@/lib/hooks/use-trade-note';

// interface TradeNotesProps {
//   docId: string; // unique id per trade (e.g., `trade-note-${tradeId}`)
//   tradeType: 'stock' | 'option';
//   tradeId: number;
//   initialContent?: string;
//   onChange?: (content: string) => void;
//   onTitleChange?: (title: string) => void;
//   className?: string;
//   autoSave?: boolean; // Enable auto-save (default: true)
//   debounceMs?: number; // Debounce delay for auto-save (default: 1000ms)
// }

// export default function TradeNotes({
//   docId,
//   tradeType,
//   tradeId,
//   initialContent,
//   onChange,
//   onTitleChange,
//   className,
//   autoSave = true,
//   debounceMs = 1000,
// }: TradeNotesProps) {
//   const {
//     note,
//     isLoading,
//     error,
//     upsert,
//     isUpserting,
//   } = useTradeNote(tradeType, tradeId, true, debounceMs);

//   // Determine the content to display
//   const displayContent = useMemo(() => {
//     // Prioritize fetched note content
//     if (note?.content) {
//       return note.content;
//     }
//     // Fall back to initialContent if provided
//     if (initialContent) {
//       return initialContent;
//     }
//     return '';
//   }, [note?.content, initialContent]);

//   // Handle content changes from BlockEditor
//   const handleChange = React.useCallback(
//     (content: string) => {
//       // Call onChange callback if provided
//       onChange?.(content);

//       // Auto-save to database if enabled
//       if (autoSave && content.trim().length > 0) {
//         upsert({ content });
//       }
//     },
//     [onChange, upsert, autoSave]
//   );

//   // Handle title changes
//   const handleTitleChange = React.useCallback(
//     (title: string) => {
//       onTitleChange?.(title);
//     },
//     [onTitleChange]
//   );

//   // Show loading state
//   if (isLoading) {
//     return (
//       <div className={cn('flex items-center justify-center', className)}>
//         <div className="text-sm text-muted-foreground">Loading note...</div>
//       </div>
//     );
//   }

//   // Show error state (non-blocking, still show editor)
//   const showError = Boolean(error && !note);

//   return (
//     <div className={cn('flex flex-col', className)}>
//       {showError ? (
//         <div className="mb-2 rounded-md bg-destructive/10 p-2 text-sm text-destructive flex-shrink-0">
//           Failed to load note. Changes will be saved when you type.
//         </div>
//       ) : null}
//       {isUpserting ? (
//         <div className="mb-2 text-xs text-muted-foreground flex-shrink-0">Saving...</div>
//       ) : null}
//       <div className="flex-1 min-h-0">
//         <BlockEditor
//           docId={docId}
//           initialContent={displayContent}
//           onChange={handleChange}
//           onTitleChange={handleTitleChange}
//           editable
//         />
//       </div>
//     </div>
//   );
// }
