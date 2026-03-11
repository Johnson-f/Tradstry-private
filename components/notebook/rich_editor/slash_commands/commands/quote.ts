import { $createQuoteNode } from '@lexical/rich-text';
import { $getSelection, $isRangeSelection } from 'lexical';
import { $setBlocksType } from '@lexical/selection';

import type { SlashCommandConfig } from '../types';

export const quoteCommand: SlashCommandConfig = {
  key: 'quote',
  title: 'Quote',
  description: 'Block quote',
  keywords: ['quote', 'blockquote'],
  execute: (editor) => {
    editor.update(() => {
      const selection = $getSelection();
      if (!$isRangeSelection(selection)) {
        return;
      }
      $setBlocksType(selection, () => $createQuoteNode());
    });
  },
};


