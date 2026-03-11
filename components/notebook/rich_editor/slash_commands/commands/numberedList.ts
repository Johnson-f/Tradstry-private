import { insertList } from '@lexical/list';

import type { SlashCommandConfig } from '../types';

export const numberedListCommand: SlashCommandConfig = {
  key: 'numbered_list',
  title: 'Numbered List',
  description: 'Convert to numbers',
  keywords: ['list', 'ol', 'numbered'],
  execute: (editor) => {
    insertList(editor, 'number');
    editor.focus();
  },
};