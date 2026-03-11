"use client";

import { useState, useEffect, useCallback } from "react";
import { AppPageHeader } from "@/components/app-page-header";
import { RichEditor } from "@/components/notebook";
import { ManageNotebooksDrawer } from "@/components/notebook/manage_notebook/drawer";
import { CalendarView } from "@/components/notebook/manage_notebook/calendar";
import { 
  CollaborationHeader,
  useSyncErrorToast,
} from "@/components/notebook/collaboration";
import {
  saveNotebookContent,
  loadNotebookContent,
  saveNotebookTitle,
  loadNotebookTitle,
  startAutoSync,
  stopAutoSync,
  saveLastOpenedNote,
} from "@/components/notebook/localstorage";
import { useNotebookNote, useNotebookNotes } from "@/lib/hooks/use-notebook";
import { useYjsCollaboration } from "@/hooks/use-yjs-collaboration";
import { useAuth } from "@/lib/hooks/use-auth";
import type { CollaboratorPresence } from "@/lib/types/collaboration";

export default function NotebookPage() {
  const [currentNoteId, setCurrentNoteId] = useState<string | undefined>(undefined);
  const [value, setValue] = useState<string | undefined>(undefined);
  const [title, setTitle] = useState<string>("");
  const [isLoaded, setIsLoaded] = useState(false);
  const [initialLoadDone, setInitialLoadDone] = useState(false);
  const [showCalendar, setShowCalendar] = useState(false);

  // Fetch notes to find the most recently updated one (backend returns sorted by updated_at desc)
  const { data: recentNotes, isLoading: notesLoading } = useNotebookNotes({ limit: 1 });

  // Restore last updated note from database on mount
  useEffect(() => {
    if (!initialLoadDone && !notesLoading) {
      if (recentNotes && recentNotes.length > 0) {
        // Backend returns notes sorted by updated_at desc, so first one is most recent
        setCurrentNoteId(recentNotes[0].id);
      }
      setInitialLoadDone(true);
    }
  }, [initialLoadDone, notesLoading, recentNotes]);

  // Fetch note from backend when noteId changes
  const { data: noteData, isLoading: noteLoading } = useNotebookNote(currentNoteId || "");

  // Load content from localStorage or backend note
  useEffect(() => {
    // Wait for initial load check to complete
    if (!initialLoadDone) return;

    if (currentNoteId && noteData) {
      // Load from backend note
      const content = JSON.stringify(noteData.content);
      setValue(content);
      setTitle(noteData.title);
      // Also save to localStorage for offline access
      saveNotebookContent(content, currentNoteId);
      saveNotebookTitle(noteData.title, currentNoteId);
      setIsLoaded(true);
    } else if (currentNoteId && !noteLoading) {
      // Note ID exists but no data yet and not loading - try localStorage first
      const savedContent = loadNotebookContent(currentNoteId);
      const savedTitle = loadNotebookTitle(currentNoteId);
      if (savedContent) setValue(savedContent);
      if (savedTitle) setTitle(savedTitle);
      if (savedContent || savedTitle) setIsLoaded(true);
    } else if (!currentNoteId && initialLoadDone) {
      // No note selected - load default note from localStorage
      const savedContent = loadNotebookContent();
      const savedTitle = loadNotebookTitle();
      if (savedContent) setValue(savedContent);
      if (savedTitle) setTitle(savedTitle);
      setIsLoaded(true);
    }
  }, [currentNoteId, noteData, noteLoading, initialLoadDone]);

  // Start auto-sync when note changes
  useEffect(() => {
    startAutoSync(currentNoteId, (newNoteId) => {
      if (newNoteId && newNoteId !== currentNoteId) {
        // Note was created on backend, update state and save as last opened
        setCurrentNoteId(newNoteId);
        saveLastOpenedNote(newNoteId);
      }
    });

    return () => {
      stopAutoSync(currentNoteId);
    };
  }, [currentNoteId]);

  // Handle content changes - save instantly to localStorage
  const handleChange = useCallback(
    (content: string) => {
      setValue(content);
      saveNotebookContent(content, currentNoteId);
    },
    [currentNoteId]
  );

  // Handle title changes - save instantly to localStorage
  const handleTitleChange = useCallback(
    (newTitle: string) => {
      setTitle(newTitle);
      saveNotebookTitle(newTitle, currentNoteId);
    },
    [currentNoteId]
  );

  // Handle note selection from drawer or calendar
  const handleNoteSelect = useCallback((noteId: string) => {
    // Close calendar if open
    setShowCalendar(false);
    // Stop sync for current note
    stopAutoSync(currentNoteId);
    // Reset state for new note
    setValue(undefined);
    setTitle("");
    setIsLoaded(false);
    // Save as last opened note for restore on reload
    saveLastOpenedNote(noteId);
    // Set new note ID - this will trigger useEffect to load the note
    setCurrentNoteId(noteId);
  }, [currentNoteId]);

  // Handle opening calendar view
  const handleOpenCalendar = useCallback(() => {
    setShowCalendar(true);
  }, []);

  // Handle closing calendar view
  const handleCloseCalendar = useCallback(() => {
    setShowCalendar(false);
  }, []);

  // Handle creating a new note (reset to blank)
  const handleNewNote = useCallback(() => {
    stopAutoSync(currentNoteId);
    setCurrentNoteId(undefined);
    setValue(undefined);
    setTitle("");
    setIsLoaded(true);
    // Clear last opened note since we're starting fresh
    saveLastOpenedNote(undefined);
  }, [currentNoteId]);

  // Don't render editor until we've loaded content
  if (!initialLoadDone || notesLoading || !isLoaded || (currentNoteId && noteLoading)) {
    return (
      <div className="flex h-screen flex-col">
        <AppPageHeader
          title="Notebook"
          height="2.75rem"
          actions={<ManageNotebooksDrawer onNoteSelect={handleNoteSelect} onOpenCalendar={handleOpenCalendar} />}
        />
        <div className="flex-1 overflow-hidden flex items-center justify-center">
          <div className="animate-pulse text-muted-foreground">Loading...</div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-screen flex-col">
      <AppPageHeader
        title={showCalendar ? "Notes Calendar" : "Notebook"}
        height="2.75rem"
        actions={
          <div className="flex items-center gap-2">
            {!showCalendar && (
              <CollaborationHeader
                noteId={currentNoteId}
                noteTitle={title}
              />
            )}
            <ManageNotebooksDrawer onNoteSelect={handleNoteSelect} onOpenCalendar={handleOpenCalendar} />
          </div>
        }
      />
      <div className="flex-1 overflow-hidden p-4">
        {showCalendar ? (
          <CalendarView
            onNoteSelect={handleNoteSelect}
            onClose={handleCloseCalendar}
          />
        ) : (
          <RichEditor
            key={currentNoteId || "new"} // Force re-mount when note changes
            value={value}
            onChange={handleChange}
            title={title}
            onTitleChange={handleTitleChange}
            titlePlaceholder="New page"
            placeholder="Write, press 'space' for AI, '/' for commands..."
            className="h-full"
            noteId={currentNoteId}
          />
        )}
      </div>
    </div>
  );
}
