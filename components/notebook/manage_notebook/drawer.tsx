"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ScrollArea } from "@/components/ui/scroll-area";
import { FolderOpen, Plus, Folder, Loader2, Trash2, RotateCcw, Trash, Tag, Pencil, Tags, Calendar } from "lucide-react";
import {
  useNotebookFolders,
  useCreateFolder,
  useUpdateFolder,
  useDeleteFolder,
  useNotebookNotes,
  useCreateNote,
  useUpdateNote,
  useDeleteNote,
  useNotebookTrash,
  useRestoreNote,
  usePermanentDeleteNote,
  useNotebookTags,
  useCreateTag,
  useUpdateTag,
  useDeleteTag,
  useSetNoteTags,
} from "@/lib/hooks/use-notebook";
import type {
  NotebookFolder,
  NotebookNote,
  NotebookTag,
  CreateFolderRequest,
  UpdateFolderRequest,
  CreateNoteRequest,
  UpdateNoteRequest,
  CreateTagRequest,
  UpdateTagRequest,
} from "@/lib/types/notebook";
import { toast } from "sonner";
import { FolderItem } from "./folder-item";
import {
  FolderDialog,
  DeleteFolderDialog,
  NoteDialog,
  DeleteNoteDialog,
  RestoreNoteDialog,
  PermanentDeleteNoteDialog,
  TagDialog,
  DeleteTagDialog,
  ManageNoteTagsDialog,
} from "./dialogs";
import type { DialogMode, ManageNotebooksDrawerProps } from "./types";
import { DEFAULT_LEXICAL_STATE } from "./types";

export function ManageNotebooksDrawer({ onNoteSelect, onOpenCalendar }: ManageNotebooksDrawerProps) {
  const [isOpen, setIsOpen] = useState(false);
  const { data: folders = [], isLoading: foldersLoading } = useNotebookFolders();
  const { data: notes = [], isLoading: notesLoading } = useNotebookNotes();
  const { data: trashedNotes = [], isLoading: trashLoading } = useNotebookTrash();
  const { data: tags = [], isLoading: tagsLoading } = useNotebookTags();
  const createFolder = useCreateFolder();
  const updateFolder = useUpdateFolder();
  const deleteFolder = useDeleteFolder();
  const createNote = useCreateNote();
  const updateNote = useUpdateNote();
  const deleteNote = useDeleteNote();
  const restoreNote = useRestoreNote();
  const permanentDeleteNote = usePermanentDeleteNote();
  const createTag = useCreateTag();
  const updateTag = useUpdateTag();
  const deleteTag = useDeleteTag();
  const setNoteTags = useSetNoteTags();

  const [dialogMode, setDialogMode] = useState<DialogMode>(null);
  const [selectedFolder, setSelectedFolder] = useState<NotebookFolder | null>(null);
  const [selectedNote, setSelectedNote] = useState<NotebookNote | null>(null);
  const [selectedTag, setSelectedTag] = useState<NotebookTag | null>(null);
  const [folderName, setFolderName] = useState("");
  const [noteName, setNoteName] = useState("");
  const [tagName, setTagName] = useState("");
  const [tagColor, setTagColor] = useState("#6b7280");
  const [isFavorite, setIsFavorite] = useState(false);
  const [isPinned, setIsPinned] = useState(false);
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set());
  const [selectedTagIds, setSelectedTagIds] = useState<string[]>([]);

  const isLoading = foldersLoading || notesLoading;
  const isTrashLoading = trashLoading;
  const isTagsLoading = tagsLoading;
  const isSubmitting =
    createFolder.isPending ||
    updateFolder.isPending ||
    deleteFolder.isPending ||
    createNote.isPending ||
    updateNote.isPending ||
    deleteNote.isPending ||
    restoreNote.isPending ||
    permanentDeleteNote.isPending ||
    createTag.isPending ||
    updateTag.isPending ||
    deleteTag.isPending ||
    setNoteTags.isPending;

  const rootFolders = folders
    .filter((f) => f.parent_folder_id === null)
    .sort((a, b) => a.position - b.position || a.name.localeCompare(b.name));

  const closeDialog = () => {
    setDialogMode(null);
    setSelectedFolder(null);
    setSelectedNote(null);
    setSelectedTag(null);
    setFolderName("");
    setNoteName("");
    setTagName("");
    setTagColor("#6b7280");
    setIsFavorite(false);
    setIsPinned(false);
    setSelectedTagIds([]);
  };

  // Folder handlers
  const openCreateFolderDialog = (parentId?: string) => {
    setDialogMode("create-folder");
    setSelectedFolder(parentId ? folders.find((f) => f.id === parentId) || null : null);
    setFolderName("");
    setIsFavorite(false);
  };

  const openEditFolderDialog = (folder: NotebookFolder) => {
    setDialogMode("edit-folder");
    setSelectedFolder(folder);
    setFolderName(folder.name);
    setIsFavorite(folder.is_favorite);
  };

  const openDeleteFolderDialog = (folder: NotebookFolder) => {
    setDialogMode("delete-folder");
    setSelectedFolder(folder);
  };

  const handleCreateFolder = async () => {
    if (!folderName.trim()) {
      toast.error("Folder name is required");
      return;
    }
    try {
      const payload: CreateFolderRequest = {
        name: folderName.trim(),
        is_favorite: isFavorite,
        parent_folder_id: selectedFolder?.id,
      };
      await createFolder.mutateAsync(payload);
      toast.success("Folder created");
      closeDialog();
    } catch {
      toast.error("Failed to create folder");
    }
  };

  const handleUpdateFolder = async () => {
    if (!selectedFolder || !folderName.trim()) {
      toast.error("Folder name is required");
      return;
    }
    try {
      const payload: UpdateFolderRequest = {
        name: folderName.trim(),
        is_favorite: isFavorite,
      };
      await updateFolder.mutateAsync({ id: selectedFolder.id, payload });
      toast.success("Folder updated");
      closeDialog();
    } catch {
      toast.error("Failed to update folder");
    }
  };

  const handleDeleteFolder = async () => {
    if (!selectedFolder) return;
    try {
      await deleteFolder.mutateAsync(selectedFolder.id);
      toast.success("Folder deleted");
      closeDialog();
    } catch {
      toast.error("Failed to delete folder");
    }
  };

  const toggleFavorite = async (folder: NotebookFolder) => {
    try {
      await updateFolder.mutateAsync({
        id: folder.id,
        payload: { is_favorite: !folder.is_favorite },
      });
      toast.success(folder.is_favorite ? "Removed from favorites" : "Added to favorites");
    } catch {
      toast.error("Failed to update folder");
    }
  };

  const toggleExpanded = (folderId: string) => {
    setExpandedFolders((prev) => {
      const next = new Set(prev);
      if (next.has(folderId)) {
        next.delete(folderId);
      } else {
        next.add(folderId);
      }
      return next;
    });
  };

  // Note handlers
  const openCreateNoteDialog = (folder: NotebookFolder) => {
    setDialogMode("create-note");
    setSelectedFolder(folder);
    setNoteName("");
    setIsPinned(false);
  };

  const openEditNoteDialog = (note: NotebookNote) => {
    setDialogMode("edit-note");
    setSelectedNote(note);
    setNoteName(note.title);
    setIsPinned(note.is_pinned);
  };

  const openDeleteNoteDialog = (note: NotebookNote) => {
    setDialogMode("delete-note");
    setSelectedNote(note);
  };

  const handleCreateNote = async () => {
    if (!noteName.trim()) {
      toast.error("Note title is required");
      return;
    }
    try {
      const payload: CreateNoteRequest = {
        title: noteName.trim(),
        folder_id: selectedFolder?.id,
        content: DEFAULT_LEXICAL_STATE,
      };
      await createNote.mutateAsync(payload);
      toast.success("Note created");
      closeDialog();
    } catch {
      toast.error("Failed to create note");
    }
  };

  const handleUpdateNote = async () => {
    if (!selectedNote || !noteName.trim()) {
      toast.error("Note title is required");
      return;
    }
    try {
      const payload: UpdateNoteRequest = {
        title: noteName.trim(),
        is_pinned: isPinned,
      };
      await updateNote.mutateAsync({ id: selectedNote.id, payload });
      toast.success("Note updated");
      closeDialog();
    } catch {
      toast.error("Failed to update note");
    }
  };

  const handleDeleteNote = async () => {
    if (!selectedNote) return;
    try {
      await deleteNote.mutateAsync(selectedNote.id);
      toast.success("Note moved to trash");
      closeDialog();
    } catch {
      toast.error("Failed to delete note");
    }
  };

  const toggleNotePin = async (note: NotebookNote) => {
    try {
      await updateNote.mutateAsync({
        id: note.id,
        payload: { is_pinned: !note.is_pinned },
      });
      toast.success(note.is_pinned ? "Note unpinned" : "Note pinned");
    } catch {
      toast.error("Failed to update note");
    }
  };

  const handleNoteClick = (note: NotebookNote) => {
    if (onNoteSelect) {
      onNoteSelect(note.id);
      setIsOpen(false);
    }
  };

  // Trash handlers
  const openRestoreNoteDialog = (note: NotebookNote) => {
    setDialogMode("restore-note");
    setSelectedNote(note);
  };

  const openPermanentDeleteDialog = (note: NotebookNote) => {
    setDialogMode("permanent-delete-note");
    setSelectedNote(note);
  };

  const handleRestoreNote = async () => {
    if (!selectedNote) return;
    try {
      await restoreNote.mutateAsync(selectedNote.id);
      toast.success("Note restored");
      closeDialog();
    } catch {
      toast.error("Failed to restore note");
    }
  };

  const handlePermanentDeleteNote = async () => {
    if (!selectedNote) return;
    try {
      await permanentDeleteNote.mutateAsync(selectedNote.id);
      toast.success("Note permanently deleted");
      closeDialog();
    } catch {
      toast.error("Failed to delete note");
    }
  };

  // Tag handlers
  const openCreateTagDialog = () => {
    setDialogMode("create-tag");
    setTagName("");
    setTagColor("#6b7280");
    setIsFavorite(false);
  };

  const openEditTagDialog = (tag: NotebookTag) => {
    setDialogMode("edit-tag");
    setSelectedTag(tag);
    setTagName(tag.name);
    setTagColor(tag.color || "#6b7280");
    setIsFavorite(tag.is_favorite);
  };

  const openDeleteTagDialog = (tag: NotebookTag) => {
    setDialogMode("delete-tag");
    setSelectedTag(tag);
  };

  const handleCreateTag = async () => {
    if (!tagName.trim()) {
      toast.error("Tag name is required");
      return;
    }
    try {
      const payload: CreateTagRequest = {
        name: tagName.trim(),
        color: tagColor,
        is_favorite: isFavorite,
      };
      await createTag.mutateAsync(payload);
      toast.success("Tag created");
      closeDialog();
    } catch {
      toast.error("Failed to create tag");
    }
  };

  const handleUpdateTag = async () => {
    if (!selectedTag || !tagName.trim()) {
      toast.error("Tag name is required");
      return;
    }
    try {
      const payload: UpdateTagRequest = {
        name: tagName.trim(),
        color: tagColor,
        is_favorite: isFavorite,
      };
      await updateTag.mutateAsync({ id: selectedTag.id, payload });
      toast.success("Tag updated");
      closeDialog();
    } catch {
      toast.error("Failed to update tag");
    }
  };

  const handleDeleteTag = async () => {
    if (!selectedTag) return;
    try {
      await deleteTag.mutateAsync(selectedTag.id);
      toast.success("Tag deleted");
      closeDialog();
    } catch {
      toast.error("Failed to delete tag");
    }
  };

  // Manage note tags
  const openManageNoteTagsDialog = (note: NotebookNote) => {
    setDialogMode("manage-note-tags");
    setSelectedNote(note);
    setSelectedTagIds(note.tags || []);
  };

  const handleTagToggle = (tagId: string) => {
    setSelectedTagIds((prev) =>
      prev.includes(tagId) ? prev.filter((id) => id !== tagId) : [...prev, tagId]
    );
  };

  const handleSaveNoteTags = async () => {
    if (!selectedNote) return;
    try {
      await setNoteTags.mutateAsync({ noteId: selectedNote.id, tagIds: selectedTagIds });
      toast.success("Tags updated");
      closeDialog();
    } catch {
      toast.error("Failed to update tags");
    }
  };

  return (
    <>
      <Sheet open={isOpen} onOpenChange={setIsOpen}>
        <SheetTrigger asChild>
          <Button variant="outline" size="sm">
            <FolderOpen className="mr-2 h-4 w-4" />
            Manage Notebooks
          </Button>
        </SheetTrigger>
        <SheetContent side="right" className="w-[400px] sm:w-[540px] p-0 flex flex-col">
          <div className="border-b px-4 py-3">
            <SheetHeader className="space-y-1">
              <SheetTitle className="text-base">Manage Notebooks</SheetTitle>
              <SheetDescription className="text-xs">
                Create, organize, and manage your trading notebooks.
              </SheetDescription>
            </SheetHeader>
          </div>
          <Tabs defaultValue="notes" className="flex-1 flex flex-col overflow-hidden">
            <div className="border-b px-4">
              <TabsList className="h-9 w-full justify-start rounded-none bg-transparent p-0">
                <TabsTrigger
                  value="notes"
                  className="rounded-none border-b-2 border-transparent px-3 py-2 text-sm data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:shadow-none"
                >
                  Manage Notes
                </TabsTrigger>
                <TabsTrigger
                  value="system"
                  className="rounded-none border-b-2 border-transparent px-3 py-2 text-sm data-[state=active]:border-primary data-[state=active]:bg-transparent data-[state=active]:shadow-none"
                >
                  System
                </TabsTrigger>
              </TabsList>
            </div>

            <TabsContent value="notes" className="flex-1 flex flex-col mt-0 overflow-hidden">
              <div className="px-4 py-3 border-b flex items-center justify-between">
                <h3 className="text-sm font-medium">Folders & Notes</h3>
                <Button size="sm" variant="outline" onClick={() => openCreateFolderDialog()}>
                  <Plus className="h-4 w-4 mr-1" />
                  New Folder
                </Button>
              </div>

              <ScrollArea className="flex-1">
                <div className="p-4 space-y-1">
                  {isLoading ? (
                    <div className="flex items-center justify-center py-8">
                      <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                    </div>
                  ) : rootFolders.length === 0 ? (
                    <div className="text-center py-8">
                      <Folder className="h-10 w-10 mx-auto text-muted-foreground/50 mb-2" />
                      <p className="text-sm text-muted-foreground">
                        No folders yet. Create your first folder to organize your notes.
                      </p>
                    </div>
                  ) : (
                    rootFolders.map((folder) => (
                      <FolderItem
                        key={folder.id}
                        folder={folder}
                        notes={notes}
                        folders={folders}
                        expandedFolders={expandedFolders}
                        onToggleExpand={toggleExpanded}
                        onToggleFavorite={toggleFavorite}
                        onCreateNote={openCreateNoteDialog}
                        onCreateSubfolder={openCreateFolderDialog}
                        onEditFolder={openEditFolderDialog}
                        onDeleteFolder={openDeleteFolderDialog}
                        onNoteClick={handleNoteClick}
                        onToggleNotePin={toggleNotePin}
                        onEditNote={openEditNoteDialog}
                        onDeleteNote={openDeleteNoteDialog}
                        onManageNoteTags={openManageNoteTagsDialog}
                      />
                    ))
                  )}
                </div>
              </ScrollArea>
            </TabsContent>

            <TabsContent value="system" className="flex-1 mt-0 overflow-hidden data-[state=inactive]:hidden">
              <ScrollArea className="h-full">
                <div className="px-4 pt-3 pb-4 space-y-6">
                  {/* Calendar Section */}
                  <div>
                    <button
                      onClick={() => {
                        if (onOpenCalendar) {
                          onOpenCalendar();
                          setIsOpen(false);
                        }
                      }}
                      className="w-full flex items-center justify-between p-3 rounded-lg border bg-card hover:bg-accent/50 transition-colors"
                    >
                      <div className="flex items-center gap-2">
                        <Calendar className="h-4 w-4 text-muted-foreground" />
                        <span className="text-sm font-medium">Calendar</span>
                      </div>
                      <span className="text-xs text-muted-foreground">View notes by date →</span>
                    </button>
                  </div>

                  {/* Tags Section */}
                  <div>
                    <div className="flex items-center justify-between mb-3">
                      <div className="flex items-center gap-2">
                        <Tag className="h-4 w-4 text-muted-foreground" />
                        <h3 className="text-sm font-medium">Tags</h3>
                        {tags.length > 0 && (
                          <span className="text-xs bg-muted px-1.5 py-0.5 rounded-full">
                            {tags.length}
                          </span>
                        )}
                      </div>
                      <Button size="sm" variant="outline" onClick={openCreateTagDialog}>
                        <Plus className="h-3.5 w-3.5 mr-1" />
                        New Tag
                      </Button>
                    </div>

                    {isTagsLoading ? (
                      <div className="flex items-center justify-center py-4">
                        <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
                      </div>
                    ) : tags.length === 0 ? (
                      <div className="text-center py-4 border rounded-lg bg-muted/30">
                        <Tags className="h-6 w-6 mx-auto text-muted-foreground/50 mb-1" />
                        <p className="text-sm text-muted-foreground">No tags yet</p>
                        <p className="text-xs text-muted-foreground/70">
                          Create tags to organize your notes
                        </p>
                      </div>
                    ) : (
                      <div className="space-y-1">
                        {tags.map((tag) => (
                          <div
                            key={tag.id}
                            className="flex items-center justify-between p-2 rounded-md border bg-card hover:bg-accent/50 transition-colors"
                          >
                            <div className="flex items-center gap-2 flex-1 min-w-0">
                              <div
                                className="w-3 h-3 rounded-full flex-shrink-0"
                                style={{ backgroundColor: tag.color || "#6b7280" }}
                              />
                              <span className="text-sm truncate">{tag.name}</span>
                              {tag.is_favorite && (
                                <span className="text-xs text-amber-500">★</span>
                              )}
                            </div>
                            <div className="flex items-center gap-1 ml-2">
                              <Button
                                variant="ghost"
                                size="icon"
                                className="h-7 w-7"
                                title="Edit tag"
                                onClick={() => openEditTagDialog(tag)}
                              >
                                <Pencil className="h-3.5 w-3.5" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="icon"
                                className="h-7 w-7 text-destructive hover:text-destructive"
                                title="Delete tag"
                                onClick={() => openDeleteTagDialog(tag)}
                              >
                                <Trash2 className="h-3.5 w-3.5" />
                              </Button>
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>

                  {/* Trash Section */}
                  <div>
                    <div className="flex items-center gap-2 mb-3">
                      <Trash2 className="h-4 w-4 text-muted-foreground" />
                      <h3 className="text-sm font-medium">Trash</h3>
                      {trashedNotes.length > 0 && (
                        <span className="text-xs bg-muted px-1.5 py-0.5 rounded-full">
                          {trashedNotes.length}
                        </span>
                      )}
                    </div>

                    {isTrashLoading ? (
                      <div className="flex items-center justify-center py-4">
                        <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
                      </div>
                    ) : trashedNotes.length === 0 ? (
                      <div className="text-center py-4 border rounded-lg bg-muted/30">
                        <Trash className="h-6 w-6 mx-auto text-muted-foreground/50 mb-1" />
                        <p className="text-sm text-muted-foreground">Trash is empty</p>
                        <p className="text-xs text-muted-foreground/70">
                          Deleted notes will appear here
                        </p>
                      </div>
                    ) : (
                      <div className="space-y-1">
                        {trashedNotes.map((note) => (
                          <div
                            key={note.id}
                            className="flex items-center justify-between p-2 rounded-md border bg-card hover:bg-accent/50 transition-colors"
                          >
                            <div className="flex-1 min-w-0">
                              <p className="text-sm font-medium truncate">{note.title}</p>
                              <p className="text-xs text-muted-foreground">
                                Deleted {new Date(note.updated_at).toLocaleDateString()}
                              </p>
                            </div>
                            <div className="flex items-center gap-1 ml-2">
                              <Button
                                variant="ghost"
                                size="icon"
                                className="h-7 w-7"
                                title="Restore note"
                                onClick={() => openRestoreNoteDialog(note)}
                              >
                                <RotateCcw className="h-3.5 w-3.5" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="icon"
                                className="h-7 w-7 text-destructive hover:text-destructive"
                                title="Delete permanently"
                                onClick={() => openPermanentDeleteDialog(note)}
                              >
                                <Trash2 className="h-3.5 w-3.5" />
                              </Button>
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              </ScrollArea>
            </TabsContent>
          </Tabs>
        </SheetContent>
      </Sheet>

      <FolderDialog
        mode={dialogMode}
        selectedFolder={selectedFolder}
        folderName={folderName}
        isFavorite={isFavorite}
        isSubmitting={isSubmitting}
        onFolderNameChange={setFolderName}
        onFavoriteChange={setIsFavorite}
        onClose={closeDialog}
        onCreate={handleCreateFolder}
        onUpdate={handleUpdateFolder}
      />

      <DeleteFolderDialog
        isOpen={dialogMode === "delete-folder"}
        folderName={selectedFolder?.name || ""}
        isSubmitting={isSubmitting}
        onClose={closeDialog}
        onDelete={handleDeleteFolder}
      />

      <NoteDialog
        mode={dialogMode}
        selectedFolder={selectedFolder}
        noteName={noteName}
        isPinned={isPinned}
        isSubmitting={isSubmitting}
        onNoteNameChange={setNoteName}
        onPinnedChange={setIsPinned}
        onClose={closeDialog}
        onCreate={handleCreateNote}
        onUpdate={handleUpdateNote}
      />

      <DeleteNoteDialog
        isOpen={dialogMode === "delete-note"}
        noteTitle={selectedNote?.title || ""}
        isSubmitting={isSubmitting}
        onClose={closeDialog}
        onDelete={handleDeleteNote}
      />

      <RestoreNoteDialog
        isOpen={dialogMode === "restore-note"}
        noteTitle={selectedNote?.title || ""}
        isSubmitting={isSubmitting}
        onClose={closeDialog}
        onRestore={handleRestoreNote}
      />

      <PermanentDeleteNoteDialog
        isOpen={dialogMode === "permanent-delete-note"}
        noteTitle={selectedNote?.title || ""}
        isSubmitting={isSubmitting}
        onClose={closeDialog}
        onDelete={handlePermanentDeleteNote}
      />

      <TagDialog
        mode={dialogMode}
        selectedTag={selectedTag}
        tagName={tagName}
        tagColor={tagColor}
        isFavorite={isFavorite}
        isSubmitting={isSubmitting}
        onTagNameChange={setTagName}
        onTagColorChange={setTagColor}
        onFavoriteChange={setIsFavorite}
        onClose={closeDialog}
        onCreate={handleCreateTag}
        onUpdate={handleUpdateTag}
      />

      <DeleteTagDialog
        isOpen={dialogMode === "delete-tag"}
        tagName={selectedTag?.name || ""}
        isSubmitting={isSubmitting}
        onClose={closeDialog}
        onDelete={handleDeleteTag}
      />

      <ManageNoteTagsDialog
        isOpen={dialogMode === "manage-note-tags"}
        noteTitle={selectedNote?.title || ""}
        allTags={tags}
        selectedTagIds={selectedTagIds}
        isSubmitting={isSubmitting}
        onTagToggle={handleTagToggle}
        onClose={closeDialog}
        onSave={handleSaveNoteTags}
      />
    </>
  );
}
