import { $createHeadingNode } from '@lexical/rich-text';
import { $getSelection, $isRangeSelection } from 'lexical';
import { $setBlocksType } from '@lexical/selection';

import type { SlashCommandConfig } from '../types';

export const heading2Command: SlashCommandConfig = {
  key: 'heading2',
  title: 'Heading 2',
  description: 'Section title',
  keywords: ['h2', 'heading', 'section'],
  execute: (editor) => {
    editor.update(() => {
      const selection = $getSelection();
      if (!$isRangeSelection(selection)) {
        return;
      }
      $setBlocksType(selection, () => $createHeadingNode('h2'));
    });
  },
};


