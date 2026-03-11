'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { useLexicalComposerContext } from '@lexical/react/LexicalComposerContext';
import {
  $deleteTableColumn__EXPERIMENTAL,
  $deleteTableRow__EXPERIMENTAL,
  $getTableCellNodeFromLexicalNode,
  $insertTableColumn__EXPERIMENTAL,
  $insertTableRow__EXPERIMENTAL,
  $isTableNode,
  TableCellNode,
} from '@lexical/table';
import { $getSelection, $isRangeSelection } from 'lexical';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Button } from '@/components/ui/button';
import {
  Minus,
  Trash2,
  MoreHorizontal,
  ArrowUp,
  ArrowDown,
  ArrowLeft,
  ArrowRight,
} from 'lucide-react';

export function TableActionMenuPlugin() {
  const [editor] = useLexicalComposerContext();
  const [tableCellNode, setTableCellNode] = useState<TableCellNode | null>(null);
  const [menuPosition, setMenuPosition] = useState<{ top: number; left: number } | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    return editor.registerUpdateListener(({ editorState }) => {
      editorState.read(() => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) {
          setTableCellNode(null);
          setMenuPosition(null);
          return;
        }

        const anchorNode = selection.anchor.getNode();
        const cellNode = $getTableCellNodeFromLexicalNode(anchorNode);

        if (cellNode) {
          setTableCellNode(cellNode);
        } else {
          setTableCellNode(null);
          setMenuPosition(null);
        }
      });
    });
  }, [editor]);

  useEffect(() => {
    if (!tableCellNode) {
      setMenuPosition(null);
      return;
    }

    const updatePosition = () => {
      const cellElement = editor.getElementByKey(tableCellNode.getKey());
      if (cellElement) {
        const rect = cellElement.getBoundingClientRect();
        setMenuPosition({
          top: rect.top - 40,
          left: rect.left + rect.width / 2 - 20,
        });
      }
    };

    updatePosition();
    window.addEventListener('scroll', updatePosition, true);
    window.addEventListener('resize', updatePosition);

    return () => {
      window.removeEventListener('scroll', updatePosition, true);
      window.removeEventListener('resize', updatePosition);
    };
  }, [editor, tableCellNode]);

  const insertRowAbove = useCallback(() => {
    editor.update(() => {
      $insertTableRow__EXPERIMENTAL(false);
    });
  }, [editor]);

  const insertRowBelow = useCallback(() => {
    editor.update(() => {
      $insertTableRow__EXPERIMENTAL(true);
    });
  }, [editor]);

  const insertColumnLeft = useCallback(() => {
    editor.update(() => {
      $insertTableColumn__EXPERIMENTAL(false);
    });
  }, [editor]);

  const insertColumnRight = useCallback(() => {
    editor.update(() => {
      $insertTableColumn__EXPERIMENTAL(true);
    });
  }, [editor]);

  const deleteRow = useCallback(() => {
    editor.update(() => {
      $deleteTableRow__EXPERIMENTAL();
    });
  }, [editor]);

  const deleteColumn = useCallback(() => {
    editor.update(() => {
      $deleteTableColumn__EXPERIMENTAL();
    });
  }, [editor]);

  const deleteTable = useCallback(() => {
    editor.update(() => {
      const selection = $getSelection();
      if (!$isRangeSelection(selection)) return;

      const anchorNode = selection.anchor.getNode();
      const cellNode = $getTableCellNodeFromLexicalNode(anchorNode);
      if (!cellNode) return;

      // Traverse up to find the table node
      let node = cellNode.getParent();
      while (node && !$isTableNode(node)) {
        node = node.getParent();
      }

      if ($isTableNode(node)) {
        node.remove();
      }
    });
    setTableCellNode(null);
    setMenuPosition(null);
  }, [editor]);

  if (!tableCellNode || !menuPosition) {
    return null;
  }

  return (
    <div
      ref={menuRef}
      className="fixed z-50"
      style={{
        top: menuPosition.top,
        left: menuPosition.left,
      }}
    >
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="outline"
            size="sm"
            className="h-8 w-8 p-0 bg-background shadow-md border"
          >
            <MoreHorizontal className="h-4 w-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="center" className="w-48">
          <DropdownMenuItem onClick={insertRowAbove}>
            <ArrowUp className="mr-2 h-4 w-4" />
            Insert row above
          </DropdownMenuItem>
          <DropdownMenuItem onClick={insertRowBelow}>
            <ArrowDown className="mr-2 h-4 w-4" />
            Insert row below
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem onClick={insertColumnLeft}>
            <ArrowLeft className="mr-2 h-4 w-4" />
            Insert column left
          </DropdownMenuItem>
          <DropdownMenuItem onClick={insertColumnRight}>
            <ArrowRight className="mr-2 h-4 w-4" />
            Insert column right
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem onClick={deleteRow} className="text-orange-600">
            <Minus className="mr-2 h-4 w-4" />
            Delete row
          </DropdownMenuItem>
          <DropdownMenuItem onClick={deleteColumn} className="text-orange-600">
            <Minus className="mr-2 h-4 w-4" />
            Delete column
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem onClick={deleteTable} className="text-destructive">
            <Trash2 className="mr-2 h-4 w-4" />
            Delete table
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
