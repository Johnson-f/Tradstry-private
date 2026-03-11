'use client';

import { useState } from 'react';
import { Share2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ShareDialog } from './share-dialog';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';

interface ShareButtonProps {
  noteId?: string;
  noteTitle: string;
  disabled?: boolean;
}

export function ShareButton({ noteId, noteTitle, disabled }: ShareButtonProps) {
  const [open, setOpen] = useState(false);

  if (!noteId) {
    return (
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="sm" disabled className="gap-2">
              <Share2 className="h-4 w-4" />
              <span className="hidden sm:inline">Share</span>
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <p>Save the note first to share</p>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  return (
    <>
      <Button
        variant="ghost"
        size="sm"
        onClick={() => setOpen(true)}
        disabled={disabled}
        className="gap-2"
      >
        <Share2 className="h-4 w-4" />
        <span className="hidden sm:inline">Share</span>
      </Button>
      <ShareDialog
        noteId={noteId}
        noteTitle={noteTitle || 'Untitled'}
        open={open}
        onOpenChange={setOpen}
      />
    </>
  );
}
