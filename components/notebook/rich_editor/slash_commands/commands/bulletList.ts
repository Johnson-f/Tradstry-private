import { insertList } from '@lexical/list';

import type { SlashCommandConfig } from '../types';

export const bulletListCommand: SlashCommandConfig = {
  key: 'bullet_list',
  title: 'Bulleted List',
  description: 'Convert to bullets',
  keywords: ['list', 'ul', 'bullet'],
  execute: (editor) => {
    insertList(editor, 'bullet');
  },
};


