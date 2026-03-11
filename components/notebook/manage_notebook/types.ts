import type { NotebookFolder, NotebookNote } from "@/lib/types/notebook";

export type DialogMode =
  | "create-folder"
  | "edit-folder"
  | "delete-folder"
  | "create-note"
  | "edit-note"
  | "delete-note"
  | "restore-note"
  | "permanent-delete-note"
  | "create-tag"
  | "edit-tag"
  | "delete-tag"
  | "manage-note-tags"
  | null;

export interface ManageNotebooksDrawerProps {
  onNoteSelect?: (noteId: string) => void;
  /** Callback to open the calendar view on the main page */
  onOpenCalendar?: () => void;
}

export interface FolderItemProps {
  folder: NotebookFolder;
  depth?: number;
  notes: NotebookNote[];
  folders: NotebookFolder[];
  expandedFolders: Set<string>;
  onToggleExpand: (folderId: string) => void;
  onToggleFavorite: (folder: NotebookFolder) => void;
  onCreateNote: (folder: NotebookFolder) => void;
  onCreateSubfolder: (parentId: string) => void;
  onEditFolder: (folder: NotebookFolder) => void;
  onDeleteFolder: (folder: NotebookFolder) => void;
  onNoteClick: (note: NotebookNote) => void;
  onToggleNotePin: (note: NotebookNote) => void;
  onEditNote: (note: NotebookNote) => void;
  onDeleteNote: (note: NotebookNote) => void;
  onManageNoteTags?: (note: NotebookNote) => void;
}

export interface NoteItemProps {
  note: NotebookNote;
  onNoteClick: (note: NotebookNote) => void;
  onTogglePin: (note: NotebookNote) => void;
  onEdit: (note: NotebookNote) => void;
  onDelete: (note: NotebookNote) => void;
  onManageTags?: (note: NotebookNote) => void;
}

// Default Lexical editor state with one empty paragraph
export const DEFAULT_LEXICAL_STATE = {
  root: {
    children: [
      {
        children: [],
        direction: null,
        format: "",
        indent: 0,
        type: "paragraph",
        version: 1,
      },
    ],
    direction: null,
    format: "",
    indent: 0,
    type: "root",
    version: 1,
  },
};
