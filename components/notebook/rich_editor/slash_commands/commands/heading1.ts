import { $createHeadingNode } from '@lexical/rich-text';
import { $getSelection, $isRangeSelection } from 'lexical';
import { $setBlocksType } from '@lexical/selection';

import type { SlashCommandConfig } from '../types';

export const heading1Command: SlashCommandConfig = {
  key: 'heading1',
  title: 'Heading 1',
  description: 'Large title',
  keywords: ['h1', 'title', 'heading'],
  execute: (editor) => {
    editor.update(() => {
      const selection = $getSelection();
      if (!$isRangeSelection(selection)) {
        return;
      }
      $setBlocksType(selection, () => $createHeadingNode('h1'));
    });
    
    editor.focus();
  },
};