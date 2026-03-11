import { $insertNodes, $getSelection, $isRangeSelection } from 'lexical';
import { $createYouTubeNode, extractYouTubeVideoId } from '../../nodes/YouTubeNode';
import type { SlashCommandConfig } from '../types';

export const youtubeCommand: SlashCommandConfig = {
  key: 'youtube',
  title: 'YouTube Video',
  description: 'Embed a YouTube video',
  keywords: ['youtube', 'video', 'embed', 'media'],
  execute: (editor) => {
    const url = prompt('Enter YouTube URL or video ID:');
    if (!url) return;

    const videoId = extractYouTubeVideoId(url.trim());
    if (!videoId) {
      alert('Invalid YouTube URL. Please enter a valid YouTube link or video ID.');
      return;
    }

    editor.update(() => {
      const selection = $getSelection();
      if ($isRangeSelection(selection)) {
        const youtubeNode = $createYouTubeNode({ videoId });
        $insertNodes([youtubeNode]);
      }
    });
  },
};
