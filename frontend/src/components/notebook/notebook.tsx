"use client";

import { Add01Icon, Notebook01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  useAccountsLoading,
  useActiveAccount,
} from "@/components/accounts/hooks";
import { Button } from "@/components/ui/button";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { Skeleton } from "@/components/ui/skeleton";
import {
  useCreateNotebookNote,
  useNotebookNotes,
  useUpdateNotebookNote,
  useUploadNotebookImage,
} from "@/hooks/notebook";
import {
  createDefaultNotebookDocumentJson,
  mergeNotebookImagesIntoDocumentJson,
  NotebookEditor,
  normalizeNotebookDocumentJson,
} from "./editor";
import { ManageNotebook } from "./manage-notebook";

export function Notebook() {
  const accountsLoading = useAccountsLoading();
  const activeAccount = useActiveAccount();
  const {
    data: notes = [],
    isLoading,
    isPending,
  } = useNotebookNotes(activeAccount?.id ?? null);
  const createNoteMutation = useCreateNotebookNote();
  const uploadImageMutation = useUploadNotebookImage();
  const updateNoteMutation = useUpdateNotebookNote();
  const [selectedNoteId, setSelectedNoteId] = useState<string | null>(null);
  const lastSavedByNoteRef = useRef<Record<string, string>>({});

  useEffect(() => {
    if (notes.length === 0) {
      setSelectedNoteId(null);
      return;
    }

    const hasSelectedNote = notes.some((note) => note.id === selectedNoteId);
    if (!hasSelectedNote) {
      setSelectedNoteId(notes[0]?.id ?? null);
    }
  }, [notes, selectedNoteId]);

  const selectedNote =
    notes.find((note) => note.id === selectedNoteId) ?? notes[0] ?? null;
  const selectedNoteDocumentJson = useMemo(
    () =>
      mergeNotebookImagesIntoDocumentJson(
        selectedNote?.documentJson ?? null,
        selectedNote?.images ?? [],
      ),
    [selectedNote],
  );

  const isNotesLoading = isLoading || isPending;

  const handleCreateNote = () => {
    if (!activeAccount) {
      return;
    }

    const documentJson = createDefaultNotebookDocumentJson();

    createNoteMutation.mutate(
      {
        accountId: activeAccount.id,
        documentJson,
        tradeIds: [],
      },
      {
        onSuccess: (note) => {
          lastSavedByNoteRef.current[note.id] = note.documentJson;
          setSelectedNoteId(note.id);
        },
      },
    );
  };

  useEffect(() => {
    if (!selectedNote) {
      return;
    }

    const normalizedDocumentJson = normalizeNotebookDocumentJson(
      selectedNoteDocumentJson,
    );

    if (!normalizedDocumentJson) {
      return;
    }

    lastSavedByNoteRef.current[selectedNote.id] = normalizedDocumentJson;
  }, [selectedNote, selectedNoteDocumentJson]);

  const handleSerializedChange = (serializedEditorState: string) => {
    if (!selectedNote) {
      return;
    }

    if (lastSavedByNoteRef.current[selectedNote.id] === serializedEditorState) {
      return;
    }

    lastSavedByNoteRef.current[selectedNote.id] = serializedEditorState;
    updateNoteMutation.mutate({
      id: selectedNote.id,
      input: {
        documentJson: serializedEditorState,
        accountId: selectedNote.accountId,
        tradeIds: selectedNote.tradeIds,
      },
    });
  };

  const draftStorageKey = useMemo(
    () =>
      selectedNote
        ? `tradstry-notebook-editor-state:${selectedNote.id}`
        : "tradstry-notebook-editor-state",
    [selectedNote],
  );

  return (
    <section className="mt-10 space-y-4">
      <div className="mx-auto flex w-full max-w-5xl justify-end px-4 sm:px-6 lg:px-10">
        <ManageNotebook
          notes={notes}
          selectedNoteId={selectedNote?.id ?? null}
          activeAccountName={activeAccount?.name ?? null}
          disabled={!activeAccount}
          isCreating={createNoteMutation.isPending}
          onCreateNote={handleCreateNote}
          onSelectNote={setSelectedNoteId}
        />
      </div>
      {accountsLoading || (activeAccount && isNotesLoading) ? (
        <div className="mx-auto w-full max-w-5xl px-4 sm:px-6 lg:px-10">
          <Skeleton className="h-[42rem] rounded-[2rem]" />
        </div>
      ) : !activeAccount ? (
        <div className="mx-auto w-full max-w-5xl px-4 sm:px-6 lg:px-10">
          <Empty className="min-h-[42rem] rounded-[2rem] border border-slate-200 bg-white">
            <EmptyHeader>
              <EmptyMedia variant="icon" className="size-12 rounded-xl">
                <HugeiconsIcon icon={Notebook01Icon} strokeWidth={2} />
              </EmptyMedia>
              <EmptyTitle className="text-base font-semibold text-slate-950">
                No active account
              </EmptyTitle>
              <EmptyDescription className="text-sm text-slate-500">
                Select or create an account before opening the notebook editor.
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
        </div>
      ) : notes.length === 0 ? (
        <div className="mx-auto w-full max-w-5xl px-4 sm:px-6 lg:px-10">
          <Empty className="min-h-[42rem] rounded-[2rem] border border-slate-200 bg-white">
            <EmptyHeader>
              <EmptyMedia variant="icon" className="size-12 rounded-xl">
                <HugeiconsIcon icon={Notebook01Icon} strokeWidth={2} />
              </EmptyMedia>
              <EmptyTitle className="text-base font-semibold text-slate-950">
                No notes yet
              </EmptyTitle>
              <EmptyDescription className="text-sm text-slate-500">
                Create your first note for {activeAccount.name} to start writing
                headers, ideas, and tagged trade context.
              </EmptyDescription>
            </EmptyHeader>
            <EmptyContent>
              <Button
                type="button"
                size="lg"
                onClick={handleCreateNote}
                disabled={createNoteMutation.isPending}
              >
                <HugeiconsIcon icon={Add01Icon} strokeWidth={2} />
                {createNoteMutation.isPending ? "Creating..." : "Create Note"}
              </Button>
            </EmptyContent>
          </Empty>
        </div>
      ) : (
        <NotebookEditor
          key={selectedNote?.id ?? "notebook-editor"}
          initialDocumentJson={selectedNoteDocumentJson}
          draftStorageKey={draftStorageKey}
          onSerializedChange={handleSerializedChange}
          onUploadImage={
            selectedNote
              ? (file) =>
                  uploadImageMutation.mutateAsync({
                    noteId: selectedNote.id,
                    file,
                  })
              : undefined
          }
        />
      )}
    </section>
  );
}
