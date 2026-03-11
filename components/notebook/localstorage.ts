import { notebookNotesService } from "@/lib/services/notebook-service";
import type { CreateNoteRequest, UpdateNoteRequest } from "@/lib/types/notebook";

const NOTEBOOK_STORAGE_KEY = "notebook_content";
const NOTEBOOK_TITLE_KEY = "notebook_title";
const NOTEBOOK_METADATA_KEY = "notebook_metadata";
const LAST_OPENED_NOTE_KEY = "notebook_last_opened";
const SYNC_INTERVAL_MS = 3 * 60 * 1000; // 3 minutes

export interface NotebookMetadata {
  lastSaved: string;
  lastSynced?: string;
  noteId?: string;
  title?: string;
  isDirty?: boolean; // Track if there are unsaved changes
}

// Track active sync timers
const syncTimers = new Map<string, NodeJS.Timeout>();
const syncCallbacks = new Map<string, () => void>();

/**
 * Save notebook content to localStorage instantly
 */
export function saveNotebookContent(content: string, noteId?: string): void {
  if (typeof window === "undefined") return;

  try {
    const key = noteId
      ? `${NOTEBOOK_STORAGE_KEY}_${noteId}`
      : NOTEBOOK_STORAGE_KEY;
    localStorage.setItem(key, content);

    // Update metadata and mark as dirty
    const metaKey = noteId
      ? `${NOTEBOOK_METADATA_KEY}_${noteId}`
      : NOTEBOOK_METADATA_KEY;
    const existingMeta = localStorage.getItem(metaKey);
    const metadata: NotebookMetadata = existingMeta
      ? { ...JSON.parse(existingMeta), lastSaved: new Date().toISOString(), isDirty: true }
      : { lastSaved: new Date().toISOString(), noteId, isDirty: true };

    localStorage.setItem(metaKey, JSON.stringify(metadata));
  } catch (error) {
    console.error("Failed to save notebook content to localStorage:", error);
  }
}

/**
 * Save notebook title to localStorage
 */
export function saveNotebookTitle(title: string, noteId?: string): void {
  if (typeof window === "undefined") return;

  try {
    const key = noteId ? `${NOTEBOOK_TITLE_KEY}_${noteId}` : NOTEBOOK_TITLE_KEY;
    localStorage.setItem(key, title);

    // Update metadata with title and mark as dirty
    const metaKey = noteId
      ? `${NOTEBOOK_METADATA_KEY}_${noteId}`
      : NOTEBOOK_METADATA_KEY;
    const existingMeta = localStorage.getItem(metaKey);
    const metadata: NotebookMetadata = existingMeta
      ? { ...JSON.parse(existingMeta), title, lastSaved: new Date().toISOString(), isDirty: true }
      : { lastSaved: new Date().toISOString(), noteId, title, isDirty: true };

    localStorage.setItem(metaKey, JSON.stringify(metadata));
  } catch (error) {
    console.error("Failed to save notebook title to localStorage:", error);
  }
}

/**
 * Load notebook content from localStorage
 */
export function loadNotebookContent(noteId?: string): string | null {
  if (typeof window === "undefined") return null;

  try {
    const key = noteId
      ? `${NOTEBOOK_STORAGE_KEY}_${noteId}`
      : NOTEBOOK_STORAGE_KEY;
    return localStorage.getItem(key);
  } catch (error) {
    console.error("Failed to load notebook content from localStorage:", error);
    return null;
  }
}

/**
 * Load notebook title from localStorage
 */
export function loadNotebookTitle(noteId?: string): string | null {
  if (typeof window === "undefined") return null;

  try {
    const key = noteId ? `${NOTEBOOK_TITLE_KEY}_${noteId}` : NOTEBOOK_TITLE_KEY;
    return localStorage.getItem(key);
  } catch (error) {
    console.error("Failed to load notebook title from localStorage:", error);
    return null;
  }
}

/**
 * Get notebook metadata from localStorage
 */
export function getNotebookMetadata(noteId?: string): NotebookMetadata | null {
  if (typeof window === "undefined") return null;

  try {
    const key = noteId
      ? `${NOTEBOOK_METADATA_KEY}_${noteId}`
      : NOTEBOOK_METADATA_KEY;
    const data = localStorage.getItem(key);
    return data ? JSON.parse(data) : null;
  } catch (error) {
    console.error("Failed to get notebook metadata from localStorage:", error);
    return null;
  }
}

/**
 * Update notebook metadata
 */
function updateMetadata(noteId: string | undefined, updates: Partial<NotebookMetadata>): void {
  if (typeof window === "undefined") return;

  try {
    const metaKey = noteId
      ? `${NOTEBOOK_METADATA_KEY}_${noteId}`
      : NOTEBOOK_METADATA_KEY;
    const existingMeta = localStorage.getItem(metaKey);
    const metadata: NotebookMetadata = existingMeta
      ? { ...JSON.parse(existingMeta), ...updates }
      : { lastSaved: new Date().toISOString(), noteId, ...updates };

    localStorage.setItem(metaKey, JSON.stringify(metadata));
  } catch (error) {
    console.error("Failed to update notebook metadata:", error);
  }
}

/**
 * Clear notebook content from localStorage
 */
export function clearNotebookContent(noteId?: string): void {
  if (typeof window === "undefined") return;

  try {
    const contentKey = noteId
      ? `${NOTEBOOK_STORAGE_KEY}_${noteId}`
      : NOTEBOOK_STORAGE_KEY;
    const titleKey = noteId
      ? `${NOTEBOOK_TITLE_KEY}_${noteId}`
      : NOTEBOOK_TITLE_KEY;
    const metaKey = noteId
      ? `${NOTEBOOK_METADATA_KEY}_${noteId}`
      : NOTEBOOK_METADATA_KEY;

    localStorage.removeItem(contentKey);
    localStorage.removeItem(titleKey);
    localStorage.removeItem(metaKey);

    // Stop any active sync timer
    stopAutoSync(noteId);
  } catch (error) {
    console.error("Failed to clear notebook content from localStorage:", error);
  }
}

/**
 * Get all saved notebook IDs from localStorage
 */
export function getSavedNotebookIds(): string[] {
  if (typeof window === "undefined") return [];

  try {
    const ids: string[] = [];
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key?.startsWith(`${NOTEBOOK_STORAGE_KEY}_`)) {
        const noteId = key.replace(`${NOTEBOOK_STORAGE_KEY}_`, "");
        if (noteId) ids.push(noteId);
      }
    }
    return ids;
  } catch (error) {
    console.error("Failed to get saved notebook IDs:", error);
    return [];
  }
}

/**
 * Extract plain text from Lexical editor JSON content
 */
function extractPlainText(content: Record<string, unknown>): string {
  try {
    const root = content.root as { children?: unknown[] } | undefined;
    if (!root?.children) return "";

    const extractText = (node: unknown): string => {
      if (!node || typeof node !== "object") return "";
      const n = node as Record<string, unknown>;

      if (n.type === "text" && typeof n.text === "string") {
        return n.text;
      }

      if (Array.isArray(n.children)) {
        return n.children.map(extractText).join(" ");
      }

      return "";
    };

    return root.children.map(extractText).join("\n").trim();
  } catch {
    return "";
  }
}

/**
 * Sync notebook to backend database
 * Creates a new note if noteId doesn't exist, otherwise updates existing note
 * Returns the noteId (existing or newly created)
 */
export async function syncToBackend(noteId?: string): Promise<string | null> {
  if (typeof window === "undefined") return null;

  try {
    const content = loadNotebookContent(noteId);
    const title = loadNotebookTitle(noteId) || "Untitled";
    const metadata = getNotebookMetadata(noteId);

    if (!content) {
      console.log("No content to sync");
      return noteId || null;
    }

    // Parse content as JSON (Lexical editor state)
    let contentJson: Record<string, unknown>;
    try {
      contentJson = JSON.parse(content);
    } catch {
      console.error("Failed to parse content as JSON");
      return noteId || null;
    }

    // Extract plain text for search/word count
    const plainText = extractPlainText(contentJson);

    if (noteId && metadata?.noteId) {
      // Update existing note
      const updatePayload: UpdateNoteRequest = {
        title,
        content: contentJson,
        content_plain_text: plainText,
      };

      const res = await notebookNotesService.update(noteId, updatePayload);
      if (res.success) {
        updateMetadata(noteId, {
          lastSynced: new Date().toISOString(),
          isDirty: false,
        });
        console.log(`Note ${noteId} synced to backend`);
        return noteId;
      } else {
        console.error("Failed to update note:", res.message);
        return noteId;
      }
    } else {
      // Create new note
      const createPayload: CreateNoteRequest = {
        title,
        content: contentJson,
        content_plain_text: plainText,
      };

      const res = await notebookNotesService.create(createPayload);
      if (res.success && res.data) {
        const newNoteId = res.data.id;

        // Migrate localStorage keys to use the new noteId
        if (!noteId) {
          // Move content from default key to note-specific key
          const oldContent = localStorage.getItem(NOTEBOOK_STORAGE_KEY);
          const oldTitle = localStorage.getItem(NOTEBOOK_TITLE_KEY);

          if (oldContent) {
            localStorage.setItem(`${NOTEBOOK_STORAGE_KEY}_${newNoteId}`, oldContent);
            localStorage.removeItem(NOTEBOOK_STORAGE_KEY);
          }
          if (oldTitle) {
            localStorage.setItem(`${NOTEBOOK_TITLE_KEY}_${newNoteId}`, oldTitle);
            localStorage.removeItem(NOTEBOOK_TITLE_KEY);
          }
          localStorage.removeItem(NOTEBOOK_METADATA_KEY);
        }

        // Update metadata with new noteId
        updateMetadata(newNoteId, {
          noteId: newNoteId,
          title,
          lastSynced: new Date().toISOString(),
          isDirty: false,
        });

        console.log(`New note created with ID: ${newNoteId}`);
        return newNoteId;
      } else {
        console.error("Failed to create note:", res.message);
        return null;
      }
    }
  } catch (error) {
    console.error("Failed to sync notebook to backend:", error);
    return noteId || null;
  }
}

/**
 * Start auto-sync timer for a notebook
 * Syncs to backend every 3 minutes if there are changes
 */
export function startAutoSync(
  noteId?: string,
  onSyncComplete?: (newNoteId: string | null) => void
): void {
  if (typeof window === "undefined") return;

  const timerKey = noteId || "default";

  // Clear existing timer if any
  stopAutoSync(noteId);

  // Store callback for this note
  if (onSyncComplete) {
    syncCallbacks.set(timerKey, () => {
      const metadata = getNotebookMetadata(noteId);
      if (metadata?.isDirty) {
        syncToBackend(noteId).then((newId) => {
          onSyncComplete(newId);
        });
      }
    });
  }

  // Start new timer
  const timer = setInterval(async () => {
    const metadata = getNotebookMetadata(noteId);

    // Only sync if there are unsaved changes
    if (metadata?.isDirty) {
      console.log(`Auto-syncing note ${noteId || "default"}...`);
      const newNoteId = await syncToBackend(noteId);

      // Call the callback with the new/existing noteId
      if (onSyncComplete) {
        onSyncComplete(newNoteId);
      }
    }
  }, SYNC_INTERVAL_MS);

  syncTimers.set(timerKey, timer);
  console.log(`Auto-sync started for note ${timerKey} (every ${SYNC_INTERVAL_MS / 1000}s)`);
}

/**
 * Stop auto-sync timer for a notebook
 */
export function stopAutoSync(noteId?: string): void {
  const timerKey = noteId || "default";
  const timer = syncTimers.get(timerKey);

  if (timer) {
    clearInterval(timer);
    syncTimers.delete(timerKey);
    syncCallbacks.delete(timerKey);
    console.log(`Auto-sync stopped for note ${timerKey}`);
  }
}

/**
 * Force immediate sync (useful for save button or before navigation)
 */
export async function forceSyncNow(noteId?: string): Promise<string | null> {
  console.log(`Force syncing note ${noteId || "default"}...`);
  return syncToBackend(noteId);
}

/**
 * Check if a note has unsaved changes
 */
export function hasUnsavedChanges(noteId?: string): boolean {
  const metadata = getNotebookMetadata(noteId);
  return metadata?.isDirty ?? false;
}

/**
 * Mark note as synced (no dirty changes)
 */
export function markAsSynced(noteId?: string): void {
  updateMetadata(noteId, {
    lastSynced: new Date().toISOString(),
    isDirty: false,
  });
}

/**
 * Get sync status for a note
 */
export function getSyncStatus(noteId?: string): {
  isDirty: boolean;
  lastSaved: string | null;
  lastSynced: string | null;
  hasBackendId: boolean;
} {
  const metadata = getNotebookMetadata(noteId);
  return {
    isDirty: metadata?.isDirty ?? false,
    lastSaved: metadata?.lastSaved ?? null,
    lastSynced: metadata?.lastSynced ?? null,
    hasBackendId: !!metadata?.noteId,
  };
}

/**
 * Save the last opened note ID to localStorage
 */
export function saveLastOpenedNote(noteId: string | undefined): void {
  if (typeof window === "undefined") return;

  try {
    if (noteId) {
      localStorage.setItem(LAST_OPENED_NOTE_KEY, noteId);
    } else {
      localStorage.removeItem(LAST_OPENED_NOTE_KEY);
    }
  } catch (error) {
    console.error("Failed to save last opened note:", error);
  }
}

/**
 * Get the last opened note ID from localStorage
 */
export function getLastOpenedNote(): string | null {
  if (typeof window === "undefined") return null;

  try {
    return localStorage.getItem(LAST_OPENED_NOTE_KEY);
  } catch (error) {
    console.error("Failed to get last opened note:", error);
    return null;
  }
}
