import type { SlashCommandConfig } from '../types';

import { bulletListCommand } from './bulletList';
import { codeBlockCommand } from './codeBlock';
import { heading1Command } from './heading1';
import { heading2Command } from './heading2';
import { heading3Command } from './heading3';
import { numberedListCommand } from './numberedList';
import { paragraphCommand } from './paragraph';
import { quoteCommand } from './quote';
import { youtubeCommand } from './youtube';
import { tweetCommand } from './tweet';
import { tableCommand } from './table';

export const slashCommandConfigs: SlashCommandConfig[] = [
  paragraphCommand,
  heading1Command,
  heading2Command,
  heading3Command,
  bulletListCommand,
  numberedListCommand,
  quoteCommand,
  codeBlockCommand,
  tableCommand,
  youtubeCommand,
  tweetCommand,
];


