import { $createHeadingNode } from '@lexical/rich-text';
import { $getSelection, $isRangeSelection } from 'lexical';
import { $setBlocksType } from '@lexical/selection';

import type { SlashCommandConfig } from '../types';

export const heading3Command: SlashCommandConfig = {
  key: 'heading3',
  title: 'Heading 3',
  keywords: ['h3', 'subheading'],
  execute: (editor) => {
    editor.update(() => {
      const selection = $getSelection();
      if (!$isRangeSelection(selection)) {
        return;
      }
      $setBlocksType(selection, () => $createHeadingNode('h3'));
    });
  },
};


