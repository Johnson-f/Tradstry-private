"use client";

import { PencilEdit01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import * as React from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useUpdatePlaybook } from "@/hooks/playbook";
import type {
  PlaybookWithStats,
  UpdatePlaybookInput,
} from "@/lib/types/playbook";

type EditPlaybookDialogProps = {
  playbook: PlaybookWithStats;
  trigger?: React.ReactNode;
};

type PlaybookFormState = {
  name: string;
  edgeName: string;
  entryRules: string;
  exitRules: string;
  positionSizingRules: string;
  additionalRules: string;
};

function createInitialState(playbook: PlaybookWithStats): PlaybookFormState {
  return {
    name: playbook.name,
    edgeName: playbook.edgeName,
    entryRules: playbook.entryRules,
    exitRules: playbook.exitRules,
    positionSizingRules: playbook.positionSizingRules,
    additionalRules: playbook.additionalRules ?? "",
  };
}

function Field({
  label,
  htmlFor,
  children,
}: {
  label: string;
  htmlFor?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="grid gap-2">
      <Label htmlFor={htmlFor}>{label}</Label>
      {children}
    </div>
  );
}

const textareaClass =
  "min-h-24 w-full rounded-md border border-input bg-input/20 px-3 py-2 text-sm outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30";

export function EditPlaybookDialog({
  playbook,
  trigger,
}: EditPlaybookDialogProps) {
  const updatePlaybook = useUpdatePlaybook();
  const [open, setOpen] = React.useState(false);
  const [form, setForm] = React.useState<PlaybookFormState>(() =>
    createInitialState(playbook),
  );
  const [error, setError] = React.useState("");

  React.useEffect(() => {
    if (open) {
      setForm(createInitialState(playbook));
      setError("");
    }
  }, [open, playbook]);

  function setField<K extends keyof PlaybookFormState>(
    key: K,
    value: PlaybookFormState[K],
  ) {
    setForm((current) => ({ ...current, [key]: value }));
    if (error) {
      setError("");
    }
  }

  function validateForm() {
    if (!form.name.trim()) return "Name is required";
    if (!form.edgeName.trim()) return "Edge name is required";
    if (!form.entryRules.trim()) return "Entry rules are required";
    if (!form.exitRules.trim()) return "Exit rules are required";
    if (!form.positionSizingRules.trim()) {
      return "Position sizing rules are required";
    }
    return "";
  }

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    const validationError = validateForm();
    if (validationError) {
      setError(validationError);
      return;
    }

    const trimmedAdditionalRules = form.additionalRules.trim();
    const input: UpdatePlaybookInput = {
      name: form.name.trim(),
      edgeName: form.edgeName.trim(),
      entryRules: form.entryRules.trim(),
      exitRules: form.exitRules.trim(),
      positionSizingRules: form.positionSizingRules.trim(),
      additionalRules: trimmedAdditionalRules || undefined,
      clearAdditionalRules: !trimmedAdditionalRules,
    };
    const toastId = toast.loading("Updating playbook...");

    try {
      await updatePlaybook.mutateAsync({
        id: playbook.id,
        input,
      });
      toast.success("Playbook updated.", { id: toastId });
      setOpen(false);
    } catch (submissionError) {
      toast.error(
        submissionError instanceof Error
          ? submissionError.message
          : "Failed to update playbook.",
        { id: toastId },
      );
      setError(
        submissionError instanceof Error
          ? submissionError.message
          : "Failed to update playbook",
      );
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        {trigger ?? (
          <Button
            variant="ghost"
            size="icon-sm"
            className="text-muted-foreground"
          >
            <HugeiconsIcon
              icon={PencilEdit01Icon}
              strokeWidth={2}
              className="size-4"
            />
            <span className="sr-only">Edit playbook</span>
          </Button>
        )}
      </DialogTrigger>

      <DialogContent className="sm:max-w-3xl">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>Edit playbook</DialogTitle>
            <DialogDescription>
              Update your setup, rules, and sizing criteria.
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-4 py-4">
            <div className="grid gap-4 md:grid-cols-2">
              <Field
                label="Playbook name"
                htmlFor={`edit-playbook-name-${playbook.id}`}
              >
                <Input
                  id={`edit-playbook-name-${playbook.id}`}
                  value={form.name}
                  onChange={(event) => setField("name", event.target.value)}
                />
              </Field>
              <Field
                label="Edge name"
                htmlFor={`edit-playbook-edge-${playbook.id}`}
              >
                <Input
                  id={`edit-playbook-edge-${playbook.id}`}
                  value={form.edgeName}
                  onChange={(event) => setField("edgeName", event.target.value)}
                />
              </Field>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <Field
                label="Entry rules"
                htmlFor={`edit-playbook-entry-${playbook.id}`}
              >
                <textarea
                  id={`edit-playbook-entry-${playbook.id}`}
                  value={form.entryRules}
                  onChange={(event) =>
                    setField("entryRules", event.target.value)
                  }
                  rows={5}
                  className={textareaClass}
                />
              </Field>
              <Field
                label="Exit rules"
                htmlFor={`edit-playbook-exit-${playbook.id}`}
              >
                <textarea
                  id={`edit-playbook-exit-${playbook.id}`}
                  value={form.exitRules}
                  onChange={(event) =>
                    setField("exitRules", event.target.value)
                  }
                  rows={5}
                  className={textareaClass}
                />
              </Field>
            </div>

            <Field
              label="Position sizing rules"
              htmlFor={`edit-playbook-position-sizing-${playbook.id}`}
            >
              <textarea
                id={`edit-playbook-position-sizing-${playbook.id}`}
                value={form.positionSizingRules}
                onChange={(event) =>
                  setField("positionSizingRules", event.target.value)
                }
                rows={5}
                className={textareaClass}
              />
            </Field>

            <Field
              label="Additional rules (optional)"
              htmlFor={`edit-playbook-additional-${playbook.id}`}
            >
              <textarea
                id={`edit-playbook-additional-${playbook.id}`}
                value={form.additionalRules}
                onChange={(event) =>
                  setField("additionalRules", event.target.value)
                }
                rows={4}
                className="min-h-20 w-full rounded-md border border-input bg-input/20 px-3 py-2 text-sm outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30"
              />
            </Field>

            {error ? <p className="text-sm text-destructive">{error}</p> : null}
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setOpen(false)}
              disabled={updatePlaybook.isPending}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={updatePlaybook.isPending}>
              {updatePlaybook.isPending ? "Saving..." : "Save changes"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
