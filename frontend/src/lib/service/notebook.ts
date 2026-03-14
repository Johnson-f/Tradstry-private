import { type GraphQLFetcher, getBackendBaseUrl } from "@/lib/client";
import type {
  CreateNotebookNoteInput,
  NotebookImage,
  NotebookNote,
  UpdateNotebookNoteInput,
} from "@/lib/types/notebook";

const NOTEBOOK_NOTE_FIELDS = `
  id
  userId
  accountId
  title
  documentJson
  tradeIds
  images {
    id
    noteId
    userId
    accountId
    cloudinaryAssetId
    cloudinaryPublicId
    secureUrl
    width
    height
    format
    bytes
    originalFilename
    createdAt
  }
  createdAt
  updatedAt
`;

const NOTEBOOK_NOTES_QUERY = `
  query NotebookNotes($accountId: String) {
    notebookNotes(accountId: $accountId) {
      ${NOTEBOOK_NOTE_FIELDS}
    }
  }
`;

const NOTEBOOK_NOTE_QUERY = `
  query NotebookNote($id: String!) {
    notebookNote(id: $id) {
      ${NOTEBOOK_NOTE_FIELDS}
    }
  }
`;

const CREATE_NOTEBOOK_NOTE_MUTATION = `
  mutation CreateNotebookNote($input: CreateNotebookNoteInput!) {
    createNotebookNote(input: $input) {
      ${NOTEBOOK_NOTE_FIELDS}
    }
  }
`;

const UPDATE_NOTEBOOK_NOTE_MUTATION = `
  mutation UpdateNotebookNote($id: String!, $input: UpdateNotebookNoteInput!) {
    updateNotebookNote(id: $id, input: $input) {
      ${NOTEBOOK_NOTE_FIELDS}
    }
  }
`;

const DELETE_NOTEBOOK_NOTE_MUTATION = `
  mutation DeleteNotebookNote($id: String!) {
    deleteNotebookNote(id: $id)
  }
`;

type TokenProvider = () => Promise<string | null>;

export async function fetchNotebookNotes(
  fetcher: GraphQLFetcher,
  accountId?: string | null,
): Promise<NotebookNote[]> {
  const data = await fetcher<{ notebookNotes: NotebookNote[] }>(
    NOTEBOOK_NOTES_QUERY,
    {
      accountId: accountId ?? null,
    },
  );
  return data.notebookNotes;
}

export async function fetchNotebookNote(
  fetcher: GraphQLFetcher,
  id: string,
): Promise<NotebookNote | null> {
  const data = await fetcher<{ notebookNote: NotebookNote | null }>(
    NOTEBOOK_NOTE_QUERY,
    { id },
  );
  return data.notebookNote;
}

export async function createNotebookNote(
  fetcher: GraphQLFetcher,
  input: CreateNotebookNoteInput,
): Promise<NotebookNote> {
  const data = await fetcher<{ createNotebookNote: NotebookNote }>(
    CREATE_NOTEBOOK_NOTE_MUTATION,
    { input },
  );
  return data.createNotebookNote;
}

export async function updateNotebookNote(
  fetcher: GraphQLFetcher,
  id: string,
  input: UpdateNotebookNoteInput,
): Promise<NotebookNote> {
  const data = await fetcher<{ updateNotebookNote: NotebookNote }>(
    UPDATE_NOTEBOOK_NOTE_MUTATION,
    { id, input },
  );
  return data.updateNotebookNote;
}

export async function deleteNotebookNote(
  fetcher: GraphQLFetcher,
  id: string,
): Promise<boolean> {
  const data = await fetcher<{ deleteNotebookNote: boolean }>(
    DELETE_NOTEBOOK_NOTE_MUTATION,
    { id },
  );
  return data.deleteNotebookNote;
}

export async function uploadNotebookImage(
  getToken: TokenProvider,
  noteId: string,
  file: File,
): Promise<NotebookImage> {
  const token = await getToken();
  const formData = new FormData();
  formData.set("noteId", noteId);
  formData.set("file", file);

  console.log("[uploadNotebookImage] uploading...", {
    noteId,
    fileName: file.name,
    fileSize: file.size,
  });

  const response = await fetch(
    `${getBackendBaseUrl()}/notebook/images/upload`,
    {
      method: "POST",
      headers: token
        ? {
            Authorization: `Bearer ${token}`,
          }
        : undefined,
      body: formData,
    },
  );

  console.log("[uploadNotebookImage] response status:", response.status);

  if (!response.ok) {
    const message = await response.text();
    console.error("[uploadNotebookImage] failed:", message);
    throw new Error(message || "Notebook image upload failed");
  }

  const data = (await response.json()) as { image: Record<string, unknown> };
  console.log("[uploadNotebookImage] success:", data);

  // Backend returns snake_case, map to camelCase
  const img = data.image;
  return {
    id: img.id,
    noteId: img.note_id,
    userId: img.user_id,
    accountId: img.account_id,
    cloudinaryAssetId: img.cloudinary_asset_id,
    cloudinaryPublicId: img.cloudinary_public_id,
    secureUrl: img.secure_url,
    width: img.width,
    height: img.height,
    format: img.format,
    bytes: img.bytes,
    originalFilename: img.original_filename,
    createdAt: img.created_at,
  } as NotebookImage;
}
