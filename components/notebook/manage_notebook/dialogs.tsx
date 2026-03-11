"use client";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Star, StarOff, Pin, PinOff, Loader2 } from "lucide-react";
import type { NotebookFolder, NotebookTag } from "@/lib/types/notebook";
import type { DialogMode } from "./types";

interface FolderDialogProps {
  mode: DialogMode;
  selectedFolder: NotebookFolder | null;
  folderName: string;
  isFavorite: boolean;
  isSubmitting: boolean;
  onFolderNameChange: (name: string) => void;
  onFavoriteChange: (favorite: boolean) => void;
  onClose: () => void;
  onCreate: () => void;
  onUpdate: () => void;
}

export function FolderDialog({
  mode,
  selectedFolder,
  folderName,
  isFavorite,
  isSubmitting,
  onFolderNameChange,
  onFavoriteChange,
  onClose,
  onCreate,
  onUpdate,
}: FolderDialogProps) {
  const isOpen = mode === "create-folder" || mode === "edit-folder";

  return (
    <Dialog open={isOpen} onOpenChange={() => onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{mode === "create-folder" ? "Create Folder" : "Edit Folder"}</DialogTitle>
          <DialogDescription>
            {mode === "create-folder"
              ? selectedFolder
                ? `Create a subfolder inside "${selectedFolder.name}"`
                : "Create a new folder to organize your notes."
              : "Update the folder name and settings."}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="folder-name">Folder Name</Label>
            <Input
              id="folder-name"
              placeholder="Enter folder name"
              value={folderName}
              onChange={(e) => onFolderNameChange(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  mode === "create-folder" ? onCreate() : onUpdate();
                }
              }}
            />
          </div>

          <div className="flex items-center gap-2">
            <Button
              type="button"
              variant={isFavorite ? "default" : "outline"}
              size="sm"
              onClick={() => onFavoriteChange(!isFavorite)}
            >
              {isFavorite ? (
                <Star className="h-4 w-4 mr-1 fill-current" />
              ) : (
                <StarOff className="h-4 w-4 mr-1" />
              )}
              {isFavorite ? "Favorite" : "Add to Favorites"}
            </Button>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button
            onClick={mode === "create-folder" ? onCreate : onUpdate}
            disabled={isSubmitting || !folderName.trim()}
          >
            {isSubmitting && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
            {mode === "create-folder" ? "Create" : "Save"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface DeleteFolderDialogProps {
  isOpen: boolean;
  folderName: string;
  isSubmitting: boolean;
  onClose: () => void;
  onDelete: () => void;
}

export function DeleteFolderDialog({
  isOpen,
  folderName,
  isSubmitting,
  onClose,
  onDelete,
}: DeleteFolderDialogProps) {
  return (
    <Dialog open={isOpen} onOpenChange={() => onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Delete Folder</DialogTitle>
          <DialogDescription>
            Are you sure you want to delete &quot;{folderName}&quot;? This will also delete all
            notes inside this folder. This action cannot be undone.
          </DialogDescription>
        </DialogHeader>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="destructive" onClick={onDelete} disabled={isSubmitting}>
            {isSubmitting && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
            Delete
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface NoteDialogProps {
  mode: DialogMode;
  selectedFolder: NotebookFolder | null;
  noteName: string;
  isPinned: boolean;
  isSubmitting: boolean;
  onNoteNameChange: (name: string) => void;
  onPinnedChange: (pinned: boolean) => void;
  onClose: () => void;
  onCreate: () => void;
  onUpdate: () => void;
}

export function NoteDialog({
  mode,
  selectedFolder,
  noteName,
  isPinned,
  isSubmitting,
  onNoteNameChange,
  onPinnedChange,
  onClose,
  onCreate,
  onUpdate,
}: NoteDialogProps) {
  const isOpen = mode === "create-note" || mode === "edit-note";

  return (
    <Dialog open={isOpen} onOpenChange={() => onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{mode === "create-note" ? "Create Note" : "Edit Note"}</DialogTitle>
          <DialogDescription>
            {mode === "create-note"
              ? `Create a new note in "${selectedFolder?.name}"`
              : "Update the note title and settings."}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="note-name">Note Title</Label>
            <Input
              id="note-name"
              placeholder="Enter note title"
              value={noteName}
              onChange={(e) => onNoteNameChange(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  mode === "create-note" ? onCreate() : onUpdate();
                }
              }}
            />
          </div>

          <div className="flex items-center gap-2">
            <Button
              type="button"
              variant={isPinned ? "default" : "outline"}
              size="sm"
              onClick={() => onPinnedChange(!isPinned)}
            >
              {isPinned ? <Pin className="h-4 w-4 mr-1 fill-current" /> : <PinOff className="h-4 w-4 mr-1" />}
              {isPinned ? "Pinned" : "Pin Note"}
            </Button>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button
            onClick={mode === "create-note" ? onCreate : onUpdate}
            disabled={isSubmitting || !noteName.trim()}
          >
            {isSubmitting && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
            {mode === "create-note" ? "Create" : "Save"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface DeleteNoteDialogProps {
  isOpen: boolean;
  noteTitle: string;
  isSubmitting: boolean;
  onClose: () => void;
  onDelete: () => void;
}

export function DeleteNoteDialog({
  isOpen,
  noteTitle,
  isSubmitting,
  onClose,
  onDelete,
}: DeleteNoteDialogProps) {
  return (
    <Dialog open={isOpen} onOpenChange={() => onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Delete Note</DialogTitle>
          <DialogDescription>
            Are you sure you want to delete &quot;{noteTitle}&quot;? The note will be moved to
            trash and can be restored later.
          </DialogDescription>
        </DialogHeader>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="destructive" onClick={onDelete} disabled={isSubmitting}>
            {isSubmitting && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
            Move to Trash
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}


interface RestoreNoteDialogProps {
  isOpen: boolean;
  noteTitle: string;
  isSubmitting: boolean;
  onClose: () => void;
  onRestore: () => void;
}

export function RestoreNoteDialog({
  isOpen,
  noteTitle,
  isSubmitting,
  onClose,
  onRestore,
}: RestoreNoteDialogProps) {
  return (
    <Dialog open={isOpen} onOpenChange={() => onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Restore Note</DialogTitle>
          <DialogDescription>
            Are you sure you want to restore &quot;{noteTitle}&quot;? The note will be moved back
            to your notebooks.
          </DialogDescription>
        </DialogHeader>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={onRestore} disabled={isSubmitting}>
            {isSubmitting && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
            Restore
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface PermanentDeleteNoteDialogProps {
  isOpen: boolean;
  noteTitle: string;
  isSubmitting: boolean;
  onClose: () => void;
  onDelete: () => void;
}

export function PermanentDeleteNoteDialog({
  isOpen,
  noteTitle,
  isSubmitting,
  onClose,
  onDelete,
}: PermanentDeleteNoteDialogProps) {
  return (
    <Dialog open={isOpen} onOpenChange={() => onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Permanently Delete Note</DialogTitle>
          <DialogDescription>
            Are you sure you want to permanently delete &quot;{noteTitle}&quot;? This action cannot
            be undone and the note will be gone forever.
          </DialogDescription>
        </DialogHeader>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="destructive" onClick={onDelete} disabled={isSubmitting}>
            {isSubmitting && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
            Delete Forever
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ============================================================================
// Tag Dialogs
// ============================================================================

const TAG_COLORS = [
  { name: "Gray", value: "#6b7280" },
  { name: "Red", value: "#ef4444" },
  { name: "Orange", value: "#f97316" },
  { name: "Amber", value: "#f59e0b" },
  { name: "Yellow", value: "#eab308" },
  { name: "Lime", value: "#84cc16" },
  { name: "Green", value: "#22c55e" },
  { name: "Emerald", value: "#10b981" },
  { name: "Teal", value: "#14b8a6" },
  { name: "Cyan", value: "#06b6d4" },
  { name: "Sky", value: "#0ea5e9" },
  { name: "Blue", value: "#3b82f6" },
  { name: "Indigo", value: "#6366f1" },
  { name: "Violet", value: "#8b5cf6" },
  { name: "Purple", value: "#a855f7" },
  { name: "Fuchsia", value: "#d946ef" },
  { name: "Pink", value: "#ec4899" },
  { name: "Rose", value: "#f43f5e" },
];

interface TagDialogProps {
  mode: DialogMode;
  selectedTag: NotebookTag | null;
  tagName: string;
  tagColor: string;
  isFavorite: boolean;
  isSubmitting: boolean;
  onTagNameChange: (name: string) => void;
  onTagColorChange: (color: string) => void;
  onFavoriteChange: (favorite: boolean) => void;
  onClose: () => void;
  onCreate: () => void;
  onUpdate: () => void;
}

export function TagDialog({
  mode,
  tagName,
  tagColor,
  isFavorite,
  isSubmitting,
  onTagNameChange,
  onTagColorChange,
  onFavoriteChange,
  onClose,
  onCreate,
  onUpdate,
}: TagDialogProps) {
  const isOpen = mode === "create-tag" || mode === "edit-tag";

  return (
    <Dialog open={isOpen} onOpenChange={() => onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{mode === "create-tag" ? "Create Tag" : "Edit Tag"}</DialogTitle>
          <DialogDescription>
            {mode === "create-tag"
              ? "Create a new tag to organize your notes."
              : "Update the tag name and color."}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="tag-name">Tag Name</Label>
            <Input
              id="tag-name"
              placeholder="Enter tag name"
              value={tagName}
              onChange={(e) => onTagNameChange(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  mode === "create-tag" ? onCreate() : onUpdate();
                }
              }}
            />
          </div>

          <div className="space-y-2">
            <Label>Color</Label>
            <div className="flex flex-wrap gap-2">
              {TAG_COLORS.map((color) => (
                <button
                  key={color.value}
                  type="button"
                  className={`w-6 h-6 rounded-full border-2 transition-all ${
                    tagColor === color.value
                      ? "border-foreground scale-110"
                      : "border-transparent hover:scale-105"
                  }`}
                  style={{ backgroundColor: color.value }}
                  onClick={() => onTagColorChange(color.value)}
                  title={color.name}
                />
              ))}
            </div>
          </div>

          <div className="flex items-center gap-2">
            <Button
              type="button"
              variant={isFavorite ? "default" : "outline"}
              size="sm"
              onClick={() => onFavoriteChange(!isFavorite)}
            >
              {isFavorite ? (
                <Star className="h-4 w-4 mr-1 fill-current" />
              ) : (
                <StarOff className="h-4 w-4 mr-1" />
              )}
              {isFavorite ? "Favorite" : "Add to Favorites"}
            </Button>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button
            onClick={mode === "create-tag" ? onCreate : onUpdate}
            disabled={isSubmitting || !tagName.trim()}
          >
            {isSubmitting && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
            {mode === "create-tag" ? "Create" : "Save"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface DeleteTagDialogProps {
  isOpen: boolean;
  tagName: string;
  isSubmitting: boolean;
  onClose: () => void;
  onDelete: () => void;
}

export function DeleteTagDialog({
  isOpen,
  tagName,
  isSubmitting,
  onClose,
  onDelete,
}: DeleteTagDialogProps) {
  return (
    <Dialog open={isOpen} onOpenChange={() => onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Delete Tag</DialogTitle>
          <DialogDescription>
            Are you sure you want to delete &quot;{tagName}&quot;? This will remove the tag from
            all notes. This action cannot be undone.
          </DialogDescription>
        </DialogHeader>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="destructive" onClick={onDelete} disabled={isSubmitting}>
            {isSubmitting && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
            Delete
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface ManageNoteTagsDialogProps {
  isOpen: boolean;
  noteTitle: string;
  allTags: NotebookTag[];
  selectedTagIds: string[];
  isSubmitting: boolean;
  onTagToggle: (tagId: string) => void;
  onClose: () => void;
  onSave: () => void;
}

export function ManageNoteTagsDialog({
  isOpen,
  noteTitle,
  allTags,
  selectedTagIds,
  isSubmitting,
  onTagToggle,
  onClose,
  onSave,
}: ManageNoteTagsDialogProps) {
  return (
    <Dialog open={isOpen} onOpenChange={() => onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Manage Tags</DialogTitle>
          <DialogDescription>
            Select tags for &quot;{noteTitle}&quot;
          </DialogDescription>
        </DialogHeader>

        <div className="py-4">
          {allTags.length === 0 ? (
            <p className="text-sm text-muted-foreground text-center py-4">
              No tags available. Create some tags first.
            </p>
          ) : (
            <div className="space-y-2 max-h-[300px] overflow-y-auto">
              {allTags.map((tag) => (
                <div
                  key={tag.id}
                  className="flex items-center gap-3 p-2 rounded-md hover:bg-accent cursor-pointer"
                  onClick={() => onTagToggle(tag.id)}
                >
                  <Checkbox
                    checked={selectedTagIds.includes(tag.id)}
                    onCheckedChange={() => onTagToggle(tag.id)}
                  />
                  <div
                    className="w-3 h-3 rounded-full"
                    style={{ backgroundColor: tag.color || "#6b7280" }}
                  />
                  <span className="text-sm">{tag.name}</span>
                </div>
              ))}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={onSave} disabled={isSubmitting}>
            {isSubmitting && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
            Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
