export interface NotebookImage {
  id: string;
  noteId: string;
  userId: string;
  accountId: string;
  cloudinaryAssetId: string;
  cloudinaryPublicId: string;
  secureUrl: string;
  width: number;
  height: number;
  format: string;
  bytes: number;
  originalFilename: string;
  createdAt: string;
}

export interface NotebookNote {
  id: string;
  userId: string;
  accountId: string;
  title: string;
  documentJson: string;
  tradeIds: string[];
  images: NotebookImage[];
  createdAt: string;
  updatedAt: string;
}

export interface CreateNotebookNoteInput {
  accountId: string;
  documentJson: string;
  tradeIds?: string[];
}

export interface UpdateNotebookNoteInput {
  accountId?: string;
  documentJson?: string;
  tradeIds?: string[];
}
