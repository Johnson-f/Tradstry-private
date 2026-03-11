"use client";

import { Button } from "@/components/ui/button";
import { FileText, Pin, PinOff, Pencil, Trash2, Tag } from "lucide-react";
import type { NoteItemProps } from "./types";

export function NoteItem({ note, onNoteClick, onTogglePin, onEdit, onDelete, onManageTags }: NoteItemProps) {
  const hasTags = note.tags && note.tags.length > 0;

  return (
    <div
      className="group flex items-center gap-2 rounded-md px-2 py-1.5 hover:bg-muted/50 transition-colors ml-6 cursor-pointer"
      onClick={() => onNoteClick(note)}
    >
      <FileText className="h-4 w-4 text-muted-foreground flex-shrink-0" />
      <span className="flex-1 text-sm truncate">{note.title}</span>
      {hasTags && (
        <span className="text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded">
          {note.tags!.length} tag{note.tags!.length !== 1 ? "s" : ""}
        </span>
      )}
      {note.is_pinned && <Pin className="h-3 w-3 text-blue-500" />}
      <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={(e) => {
            e.stopPropagation();
            onTogglePin(note);
          }}
        >
          {note.is_pinned ? (
            <PinOff className="h-3.5 w-3.5 text-blue-500" />
          ) : (
            <Pin className="h-3.5 w-3.5 text-muted-foreground" />
          )}
        </Button>
        {onManageTags && (
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            title="Manage tags"
            onClick={(e) => {
              e.stopPropagation();
              onManageTags(note);
            }}
          >
            <Tag className="h-3.5 w-3.5 text-muted-foreground" />
          </Button>
        )}
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={(e) => {
            e.stopPropagation();
            onEdit(note);
          }}
        >
          <Pencil className="h-3.5 w-3.5" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6 text-destructive hover:text-destructive"
          onClick={(e) => {
            e.stopPropagation();
            onDelete(note);
          }}
        >
          <Trash2 className="h-3.5 w-3.5" />
        </Button>
      </div>
    </div>
  );
}
