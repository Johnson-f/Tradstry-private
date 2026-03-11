import { $insertNodes, $getSelection, $isRangeSelection } from 'lexical';
import { $createTweetNode, extractTweetId } from '../../nodes/TweetNode';
import type { SlashCommandConfig } from '../types';

export const tweetCommand: SlashCommandConfig = {
  key: 'tweet',
  title: 'Tweet / X Post',
  description: 'Embed a tweet from Twitter/X',
  keywords: ['tweet', 'twitter', 'x', 'social', 'embed'],
  execute: (editor) => {
    const url = prompt('Enter Tweet URL or ID:');
    if (!url) return;

    const tweetId = extractTweetId(url.trim());
    if (!tweetId) {
      alert('Invalid Tweet URL. Please enter a valid Twitter/X link or tweet ID.');
      return;
    }

    editor.update(() => {
      const selection = $getSelection();
      if ($isRangeSelection(selection)) {
        const tweetNode = $createTweetNode({ tweetId });
        $insertNodes([tweetNode]);
      }
    });
  },
};
