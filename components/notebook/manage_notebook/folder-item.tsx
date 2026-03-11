"use client";

import { Button } from "@/components/ui/button";
import {
  Folder,
  ChevronRight,
  Star,
  StarOff,
  FileText,
  Plus,
  Pencil,
  Trash2,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { NoteItem } from "./note-item";
import type { FolderItemProps } from "./types";

export function FolderItem({
  folder,
  depth = 0,
  notes,
  folders,
  expandedFolders,
  onToggleExpand,
  onToggleFavorite,
  onCreateNote,
  onCreateSubfolder,
  onEditFolder,
  onDeleteFolder,
  onNoteClick,
  onToggleNotePin,
  onEditNote,
  onDeleteNote,
  onManageNoteTags,
}: FolderItemProps) {
  const children = folders.filter((f) => f.parent_folder_id === folder.id);
  const folderNotes = notes.filter((n) => n.folder_id === folder.id && !n.is_deleted);
  const isExpanded = expandedFolders.has(folder.id);

  return (
    <div>
      <div
        className={cn(
          "group flex items-center gap-2 rounded-md px-2 py-1.5 hover:bg-muted/50 transition-colors",
          depth > 0 && "ml-4"
        )}
      >
        <button onClick={() => onToggleExpand(folder.id)} className="p-0.5 hover:bg-muted rounded">
          <ChevronRight
            className={cn(
              "h-3.5 w-3.5 text-muted-foreground transition-transform",
              isExpanded && "rotate-90"
            )}
          />
        </button>

        <Folder className="h-4 w-4 text-muted-foreground flex-shrink-0" />

        <span className="flex-1 text-sm truncate">
          {folder.name}
          {folderNotes.length > 0 && (
            <span className="ml-1 text-xs text-muted-foreground">({folderNotes.length})</span>
          )}
        </span>

        <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
          <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => onToggleFavorite(folder)}>
            {folder.is_favorite ? (
              <Star className="h-3.5 w-3.5 text-yellow-500 fill-yellow-500" />
            ) : (
              <StarOff className="h-3.5 w-3.5 text-muted-foreground" />
            )}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            onClick={() => onCreateNote(folder)}
            title="New Note"
          >
            <FileText className="h-3.5 w-3.5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            onClick={() => onCreateSubfolder(folder.id)}
            title="New Subfolder"
          >
            <Plus className="h-3.5 w-3.5" />
          </Button>
          <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => onEditFolder(folder)}>
            <Pencil className="h-3.5 w-3.5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6 text-destructive hover:text-destructive"
            onClick={() => onDeleteFolder(folder)}
          >
            <Trash2 className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>

      {isExpanded && (
        <div className="border-l border-border ml-[18px]">
          {/* Pinned notes first */}
          {folderNotes
            .filter((n) => n.is_pinned)
            .map((note) => (
              <NoteItem
                key={note.id}
                note={note}
                onNoteClick={onNoteClick}
                onTogglePin={onToggleNotePin}
                onEdit={onEditNote}
                onDelete={onDeleteNote}
                onManageTags={onManageNoteTags}
              />
            ))}
          {/* Then unpinned notes */}
          {folderNotes
            .filter((n) => !n.is_pinned)
            .map((note) => (
              <NoteItem
                key={note.id}
                note={note}
                onNoteClick={onNoteClick}
                onTogglePin={onToggleNotePin}
                onEdit={onEditNote}
                onDelete={onDeleteNote}
                onManageTags={onManageNoteTags}
              />
            ))}
          {/* Child folders */}
          {children.map((child) => (
            <FolderItem
              key={child.id}
              folder={child}
              depth={depth + 1}
              notes={notes}
              folders={folders}
              expandedFolders={expandedFolders}
              onToggleExpand={onToggleExpand}
              onToggleFavorite={onToggleFavorite}
              onCreateNote={onCreateNote}
              onCreateSubfolder={onCreateSubfolder}
              onEditFolder={onEditFolder}
              onDeleteFolder={onDeleteFolder}
              onNoteClick={onNoteClick}
              onToggleNotePin={onToggleNotePin}
              onEditNote={onEditNote}
              onDeleteNote={onDeleteNote}
              onManageNoteTags={onManageNoteTags}
            />
          ))}
        </div>
      )}
    </div>
  );
}
