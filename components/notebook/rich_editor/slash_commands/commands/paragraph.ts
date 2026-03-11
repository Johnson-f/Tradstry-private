import { $createParagraphNode, $getSelection, $isRangeSelection } from 'lexical';
import { $setBlocksType } from '@lexical/selection';

import type { SlashCommandConfig } from '../types';

export const paragraphCommand: SlashCommandConfig = {
  key: 'paragraph',
  title: 'Paragraph',
  keywords: ['text', 'p'],
  execute: (editor) => {
    editor.update(() => {
      const selection = $getSelection();
      if (!$isRangeSelection(selection)) {
        return;
      }
      $setBlocksType(selection, () => $createParagraphNode());
    });
  },
};


