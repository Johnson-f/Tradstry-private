"use client";

import {
  Add01Icon,
  Cancel01Icon,
  Note01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Button } from "@/components/ui/button";
import {
  Drawer,
  DrawerClose,
  DrawerContent,
  DrawerDescription,
  DrawerHeader,
  DrawerTitle,
  DrawerTrigger,
} from "@/components/ui/drawer";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { NotebookNote } from "@/lib/types/notebook";
import { cn } from "@/lib/utils";

export function ManageNotebook({
  notes,
  selectedNoteId,
  activeAccountName,
  disabled = false,
  isCreating = false,
  onCreateNote,
  onSelectNote,
}: {
  notes: NotebookNote[];
  selectedNoteId: string | null;
  activeAccountName: string | null;
  disabled?: boolean;
  isCreating?: boolean;
  onCreateNote: () => void;
  onSelectNote: (noteId: string) => void;
}) {
  return (
    <Drawer direction="right">
      <DrawerTrigger asChild>
        <Button type="button" variant="outline" size="lg" disabled={disabled}>
          Manage Notes
        </Button>
      </DrawerTrigger>
      <DrawerContent className="w-full max-w-md p-0 before:inset-0 before:rounded-none before:border-l">
        <DrawerClose asChild>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            className="absolute top-4 left-4 z-10"
          >
            <HugeiconsIcon icon={Cancel01Icon} strokeWidth={2} />
            <span className="sr-only">Close</span>
          </Button>
        </DrawerClose>
        <DrawerHeader className="border-b border-slate-200 px-6 py-5 pl-14">
          <DrawerTitle className="text-base font-semibold text-slate-950">
            Manage Notes
          </DrawerTitle>
          <DrawerDescription className="text-sm text-slate-500">
            {activeAccountName
              ? `Organize notes for ${activeAccountName}.`
              : "Select an account to manage notes."}
          </DrawerDescription>
        </DrawerHeader>
        <div className="border-b border-slate-200 px-6 py-4">
          <Button
            type="button"
            className="w-full justify-center"
            onClick={onCreateNote}
            disabled={disabled || isCreating}
          >
            <HugeiconsIcon icon={Add01Icon} strokeWidth={2} />
            {isCreating ? "Creating..." : "Create Note"}
          </Button>
        </div>
        <ScrollArea className="flex-1">
          <div className="px-3 py-3">
            {notes.length === 0 ? (
              <div className="rounded-xl border border-dashed border-slate-200 px-4 py-6 text-center text-sm leading-6 text-slate-500">
                No notes yet.
              </div>
            ) : (
              <div className="space-y-2">
                {notes.map((note) => (
                  <button
                    key={note.id}
                    type="button"
                    className={cn(
                      "flex w-full items-start gap-3 rounded-xl border px-4 py-3 text-left transition-colors",
                      selectedNoteId === note.id
                        ? "border-slate-900 bg-slate-900 text-white"
                        : "border-slate-200 bg-white text-slate-900 hover:bg-slate-50",
                    )}
                    onClick={() => onSelectNote(note.id)}
                  >
                    <span
                      className={cn(
                        "mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg",
                        selectedNoteId === note.id
                          ? "bg-white/10 text-white"
                          : "bg-slate-100 text-slate-700",
                      )}
                    >
                      <HugeiconsIcon icon={Note01Icon} strokeWidth={2} />
                    </span>
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-sm font-medium">
                        {note.title}
                      </span>
                      <span
                        className={cn(
                          "mt-1 block text-xs",
                          selectedNoteId === note.id
                            ? "text-slate-300"
                            : "text-slate-500",
                        )}
                      >
                        {new Date(note.updatedAt).toLocaleString()}
                      </span>
                    </span>
                  </button>
                ))}
              </div>
            )}
          </div>
        </ScrollArea>
      </DrawerContent>
    </Drawer>
  );
}
