"use client";

import { HugeiconsIcon } from "@hugeicons/react";
import * as React from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useAccountActions, useAccounts } from "./hooks";
import { ACCOUNT_ICONS, DEFAULT_ICON, ICON_OPTIONS } from "./icon-map";
import type { Account, Currency, RiskProfile } from "./types";
import { CURRENCIES } from "./types";

interface AccountDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  account?: Account | null;
}

export function AccountDialog({
  open,
  onOpenChange,
  account,
}: AccountDialogProps) {
  const isEditing = !!account;
  const accounts = useAccounts();
  const actions = useAccountActions();

  const [name, setName] = React.useState("");
  const [currency, setCurrency] = React.useState<Currency>("USD");
  const [riskProfile, setRiskProfile] = React.useState<RiskProfile>("moderate");
  const [icon, setIcon] = React.useState(DEFAULT_ICON);
  const [error, setError] = React.useState("");

  // biome-ignore lint/correctness/useExhaustiveDependencies: reset form state when dialog opens
  React.useEffect(() => {
    if (account) {
      setName(account.name);
      setCurrency(account.currency);
      setRiskProfile(account.riskProfile);
      setIcon(account.icon);
    } else {
      setName("");
      setCurrency("USD");
      setRiskProfile("moderate");
      setIcon(DEFAULT_ICON);
    }
    setError("");
  }, [account, open]);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();

    const trimmedName = name.trim();
    if (!trimmedName) {
      setError("Account name is required");
      return;
    }
    if (trimmedName.length > 50) {
      setError("Account name must be 50 characters or less");
      return;
    }

    const isDuplicate = accounts.some(
      (a) =>
        a.name.toLowerCase() === trimmedName.toLowerCase() &&
        a.id !== account?.id,
    );
    if (isDuplicate) {
      setError("An account with this name already exists");
      return;
    }

    if (isEditing && account) {
      actions.update(account.id, {
        name: trimmedName,
        currency,
        riskProfile,
        icon,
      });
    } else {
      actions.create({
        name: trimmedName,
        currency,
        riskProfile,
        icon,
        broker: null,
      });
    }

    onOpenChange(false);
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>
              {isEditing ? "Edit Account" : "Create Account"}
            </DialogTitle>
            <DialogDescription>
              {isEditing
                ? "Update your trading account settings."
                : "Set up a new trading portfolio."}
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <Label htmlFor="account-name">Name</Label>
              <Input
                id="account-name"
                value={name}
                onChange={(e) => {
                  setName(e.target.value);
                  setError("");
                }}
                placeholder="e.g., Main Portfolio"
                maxLength={50}
              />
              {error && <p className="text-sm text-destructive">{error}</p>}
            </div>

            <div className="grid gap-2">
              <Label>Currency</Label>
              <Select
                value={currency}
                onValueChange={(v) => setCurrency(v as Currency)}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {CURRENCIES.map((c) => (
                    <SelectItem key={c} value={c}>
                      {c}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="grid gap-2">
              <Label>Risk Profile</Label>
              <RadioGroup
                value={riskProfile}
                onValueChange={(v) => setRiskProfile(v as RiskProfile)}
                className="flex gap-4"
              >
                <div className="flex items-center gap-2">
                  <RadioGroupItem value="conservative" id="risk-conservative" />
                  <Label htmlFor="risk-conservative" className="font-normal">
                    Conservative
                  </Label>
                </div>
                <div className="flex items-center gap-2">
                  <RadioGroupItem value="moderate" id="risk-moderate" />
                  <Label htmlFor="risk-moderate" className="font-normal">
                    Moderate
                  </Label>
                </div>
                <div className="flex items-center gap-2">
                  <RadioGroupItem value="aggressive" id="risk-aggressive" />
                  <Label htmlFor="risk-aggressive" className="font-normal">
                    Aggressive
                  </Label>
                </div>
              </RadioGroup>
            </div>

            <div className="grid gap-2">
              <Label>Icon</Label>
              <div className="flex flex-wrap gap-2">
                {ICON_OPTIONS.map((key) => {
                  const iconData = ACCOUNT_ICONS[key];
                  if (!iconData) return null;
                  return (
                    <button
                      key={key}
                      type="button"
                      onClick={() => setIcon(key)}
                      className={`flex size-9 items-center justify-center rounded-md border transition-colors ${
                        icon === key
                          ? "border-primary bg-primary/10 text-primary"
                          : "border-border hover:border-primary/50"
                      }`}
                    >
                      <HugeiconsIcon
                        icon={iconData}
                        strokeWidth={2}
                        className="size-4"
                      />
                    </button>
                  );
                })}
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button type="submit">
              {isEditing ? "Save Changes" : "Create Account"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
