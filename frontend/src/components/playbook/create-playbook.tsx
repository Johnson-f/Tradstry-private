"use client";

import { PlusSignIcon } from "@hugeicons/core-free-icons";
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
import { useCreatePlaybook } from "@/hooks/playbook";
import type { CreatePlaybookInput } from "@/lib/types/playbook";

type CreatePlaybookDialogProps = {
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  trigger?: React.ReactNode;
  triggerLabel?: string;
  onCreated?: () => void;
  disabled?: boolean;
};

type PlaybookFormState = {
  name: string;
  edgeName: string;
  entryRules: string;
  exitRules: string;
  positionSizingRules: string;
  additionalRules: string;
};

const initialFormState: PlaybookFormState = {
  name: "",
  edgeName: "",
  entryRules: "",
  exitRules: "",
  positionSizingRules: "",
  additionalRules: "",
};

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

export function CreatePlaybookDialog({
  open,
  onOpenChange,
  trigger,
  triggerLabel = "Create Playbook",
  onCreated,
  disabled = false,
}: CreatePlaybookDialogProps) {
  const createPlaybook = useCreatePlaybook();
  const [isOpen, setIsOpen] = React.useState(false);
  const [form, setForm] = React.useState<PlaybookFormState>(initialFormState);
  const [error, setError] = React.useState("");

  const controlled = open !== undefined;
  const dialogOpen = controlled ? open : isOpen;

  function setDialogOpen(next: boolean) {
    if (!controlled) {
      setIsOpen(next);
    }
    onOpenChange?.(next);

    if (!next) {
      setForm(initialFormState);
      setError("");
    }
  }

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

    const input: CreatePlaybookInput = {
      name: form.name.trim(),
      edgeName: form.edgeName.trim(),
      entryRules: form.entryRules.trim(),
      exitRules: form.exitRules.trim(),
      positionSizingRules: form.positionSizingRules.trim(),
      additionalRules: form.additionalRules.trim() || null,
    };
    const toastId = toast.loading("Creating playbook...");

    try {
      await createPlaybook.mutateAsync(input);
      toast.success("Playbook created.", { id: toastId });
      onCreated?.();
      setDialogOpen(false);
    } catch (submissionError) {
      toast.error(
        submissionError instanceof Error
          ? submissionError.message
          : "Failed to create playbook.",
        { id: toastId },
      );
      setError(
        submissionError instanceof Error
          ? submissionError.message
          : "Failed to create playbook",
      );
    }
  }

  const textareaClass =
    "min-h-24 w-full rounded-md border border-input bg-input/20 px-3 py-2 text-sm outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30";

  return (
    <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
      {!controlled ? (
        <DialogTrigger asChild>
          {trigger ?? (
            <Button
              size="sm"
              variant="default"
              disabled={disabled}
              className="gap-2 font-semibold"
            >
              <HugeiconsIcon icon={PlusSignIcon} className="size-4" />
              {triggerLabel}
            </Button>
          )}
        </DialogTrigger>
      ) : trigger ? (
        <DialogTrigger asChild>{trigger}</DialogTrigger>
      ) : null}

      <DialogContent className="sm:max-w-3xl">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>Create playbook</DialogTitle>
            <DialogDescription>
              Define your setup, rules, and sizing criteria in one place.
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-4 py-4">
            <div className="grid gap-4 md:grid-cols-2">
              <Field label="Playbook name" htmlFor="playbook-name">
                <Input
                  id="playbook-name"
                  value={form.name}
                  onChange={(event) => setField("name", event.target.value)}
                  placeholder="Breakout Long Setup"
                />
              </Field>
              <Field label="Edge name" htmlFor="playbook-edge">
                <Input
                  id="playbook-edge"
                  value={form.edgeName}
                  onChange={(event) => setField("edgeName", event.target.value)}
                  placeholder="Gap & Follow-through"
                />
              </Field>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <Field label="Entry rules" htmlFor="playbook-entry">
                <textarea
                  id="playbook-entry"
                  value={form.entryRules}
                  onChange={(event) =>
                    setField("entryRules", event.target.value)
                  }
                  placeholder="Exact conditions to enter a trade"
                  rows={5}
                  className={textareaClass}
                />
              </Field>
              <Field label="Exit rules" htmlFor="playbook-exit">
                <textarea
                  id="playbook-exit"
                  value={form.exitRules}
                  onChange={(event) =>
                    setField("exitRules", event.target.value)
                  }
                  placeholder="Exact conditions to take profit or reduce risk"
                  rows={5}
                  className={textareaClass}
                />
              </Field>
            </div>

            <Field
              label="Position sizing rules"
              htmlFor="playbook-position-sizing"
            >
              <textarea
                id="playbook-position-sizing"
                value={form.positionSizingRules}
                onChange={(event) =>
                  setField("positionSizingRules", event.target.value)
                }
                placeholder="Position size, max risk, and pre-defined max units"
                rows={5}
                className={textareaClass}
              />
            </Field>

            <Field
              label="Additional rules (optional)"
              htmlFor="playbook-additional"
            >
              <textarea
                id="playbook-additional"
                value={form.additionalRules}
                onChange={(event) =>
                  setField("additionalRules", event.target.value)
                }
                placeholder="Optional risk management or setup context"
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
              onClick={() => setDialogOpen(false)}
              disabled={createPlaybook.isPending}
            >
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={createPlaybook.isPending || disabled}
            >
              {createPlaybook.isPending ? "Saving..." : "Save playbook"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
