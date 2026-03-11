import { $insertNodes, $getSelection, $isRangeSelection } from 'lexical';
import { $createTableNodeWithDimensions } from '../../nodes/TableNodes';
import type { SlashCommandConfig } from '../types';

export const tableCommand: SlashCommandConfig = {
  key: 'table',
  title: 'Table',
  description: 'Insert a 3×3 table',
  keywords: ['table', 'grid', 'spreadsheet', 'data'],
  execute: (editor) => {
    editor.update(() => {
      const selection = $getSelection();
      if ($isRangeSelection(selection)) {
        const tableNode = $createTableNodeWithDimensions(3, 3, true);
        $insertNodes([tableNode]);
      }
    });
  },
};
