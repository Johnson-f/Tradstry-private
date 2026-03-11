'use client';

// Re-export Lexical table nodes for use in the editor
// These are the official Lexical table nodes with full functionality

export {
  TableNode,
  TableCellNode,
  TableRowNode,
  $createTableNode,
  $createTableCellNode,
  $createTableRowNode,
  $isTableNode,
  $isTableCellNode,
  $isTableRowNode,
  $getTableCellNodeFromLexicalNode,
  $getTableRowIndexFromTableCellNode,
  $getTableColumnIndexFromTableCellNode,
} from '@lexical/table';

export type {
  SerializedTableNode,
  SerializedTableCellNode,
  SerializedTableRowNode,
  TableCellHeaderStates,
} from '@lexical/table';

// Helper to create a table with specified dimensions
import {
  $createTableNode,
  $createTableCellNode,
  $createTableRowNode,
  TableCellHeaderStates,
} from '@lexical/table';
import { $createParagraphNode, $createTextNode } from 'lexical';

export function $createTableNodeWithDimensions(
  rows: number,
  columns: number,
  includeHeaders: boolean = true
): ReturnType<typeof $createTableNode> {
  const tableNode = $createTableNode();

  for (let r = 0; r < rows; r++) {
    const rowNode = $createTableRowNode();
    
    for (let c = 0; c < columns; c++) {
      const headerState: TableCellHeaderStates =
        includeHeaders && r === 0
          ? 1 // TableCellHeaderStates.ROW
          : 0; // TableCellHeaderStates.NO_STATUS
      
      const cellNode = $createTableCellNode(headerState);
      const paragraphNode = $createParagraphNode();
      paragraphNode.append($createTextNode(''));
      cellNode.append(paragraphNode);
      rowNode.append(cellNode);
    }
    
    tableNode.append(rowNode);
  }

  return tableNode;
}
