import { $createCodeNode, $isCodeNode } from '@lexical/code';
import { $getSelection, $isRangeSelection } from 'lexical';
import { $setBlocksType } from '@lexical/selection';

import type { SlashCommandConfig } from '../types';

export const SUPPORTED_CODE_LANGUAGES = [
  'typescript',
  'javascript',
  'python',
  'rust',
  'go',
  'java',
  'csharp',
  'cpp',
  'php',
  'ruby',
  'kotlin',
  'swift',
] as const;

const DEFAULT_CODE_LANGUAGE: (typeof SUPPORTED_CODE_LANGUAGES)[number] = 'typescript';

export const codeBlockCommand: SlashCommandConfig = {
  key: 'code_block',
  title: 'Code Block',
  description: 'Monospace block',
  keywords: ['code', 'snippet'],
  execute: (editor) => {
    editor.update(() => {
      const selection = $getSelection();
      if (!$isRangeSelection(selection)) {
        return;
      }
      $setBlocksType(selection, () => $createCodeNode());

      const nodes = selection.getNodes();
      const codeNode = nodes.find($isCodeNode);
      if (codeNode) {
        codeNode.setLanguage(DEFAULT_CODE_LANGUAGE);
      }
    });
  },
};


