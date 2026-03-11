/**
 * Playbook Create Dialog - WebSocket + REST version
 */

import { useMemo, useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { toast } from 'sonner';
import { usePlaybooks } from '@/lib/hooks/use-playbooks';
import playbookService from '@/lib/services/playbook-service';
import type { Playbook } from '@/lib/types/playbook';
import Picker from '@emoji-mart/react';
import emojiData from '@emoji-mart/data';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { HexColorPicker } from 'react-colorful';
import { colord } from 'colord';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { MoreVertical, Edit, Trash2 } from 'lucide-react';

interface PlaybookCreateDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onPlaybookCreated?: (playbook: Playbook) => void;
}

type RuleGroupKey = 'entry' | 'exit';

type EmojiSelection = {
  native?: string;
  skins?: Array<{ native?: string }>;
};

function extractNativeEmoji(selection: unknown): string | null {
  const s = selection as EmojiSelection | null | undefined;
  if (!s) return null;
  if (typeof s.native === 'string' && s.native.length > 0) return s.native;
  const skinNative = s.skins?.[0]?.native;
  return typeof skinNative === 'string' && skinNative.length > 0 ? skinNative : null;
}

export function PlaybookCreateDialog({
  open,
  onOpenChange,
  onPlaybookCreated,
}: PlaybookCreateDialogProps) {
  const { createPlaybook } = usePlaybooks();

  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [color, setColor] = useState<string>('#f59e0b');
  const [emoji, setEmoji] = useState<string>('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [errors, setErrors] = useState<string[]>([]);

  const [entryRules, setEntryRules] = useState<string[]>([]);
  const [exitRules, setExitRules] = useState<string[]>([]);

  const [isEmojiPickerOpen, setIsEmojiPickerOpen] = useState(false);
  const [isRuleDialogOpen, setIsRuleDialogOpen] = useState(false);
  const [ruleDialogGroup, setRuleDialogGroup] = useState<RuleGroupKey>('entry');
  const [ruleTitle, setRuleTitle] = useState('');
  const [editingRuleIndex, setEditingRuleIndex] = useState<number | null>(null);

  const canSubmit = useMemo(() => name.trim().length > 0, [name]);

  const addRule = (group: RuleGroupKey) => {
    setRuleDialogGroup(group);
    setRuleTitle('');
    setIsRuleDialogOpen(true);
  };

  const confirmAddRule = () => {
    const trimmed = ruleTitle.trim();
    if (!trimmed) return;
    if (editingRuleIndex !== null) {
      if (ruleDialogGroup === 'entry') {
        const updated = [...entryRules];
        updated[editingRuleIndex] = trimmed;
        setEntryRules(updated);
      } else {
        const updated = [...exitRules];
        updated[editingRuleIndex] = trimmed;
        setExitRules(updated);
      }
      setEditingRuleIndex(null);
    } else {
      if (ruleDialogGroup === 'entry') setEntryRules((r) => [...r, trimmed]);
      else setExitRules((r) => [...r, trimmed]);
    }
    setIsRuleDialogOpen(false);
    setRuleTitle('');
  };

  const handleEditRule = (group: RuleGroupKey, idx: number) => {
    setRuleDialogGroup(group);
    const title = group === 'entry' ? entryRules[idx] : exitRules[idx];
    setRuleTitle(title);
    setEditingRuleIndex(idx);
    setIsRuleDialogOpen(true);
  };

  const removeRule = (group: RuleGroupKey, idx: number) => {
    if (group === 'entry') setEntryRules((r) => r.filter((_, i) => i !== idx));
    else setExitRules((r) => r.filter((_, i) => i !== idx));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    const currentErrors: string[] = [];
    if (!name.trim()) currentErrors.push('Playbook name is required');
    if (name.length > 100) currentErrors.push('Playbook name must be less than 100 characters');
    if (description && description.length > 500) currentErrors.push('Description must be less than 500 characters');
    setErrors(currentErrors);
    if (currentErrors.length > 0) return;

    setIsSubmitting(true);
    try {
      const created = await createPlaybook({
        name: name.trim(),
        description: description || undefined,
        color,
        emoji: emoji || undefined,
      });

      // Persist rules using the newly created id
      for (const title of entryRules) {
        await playbookService.createRule(created.id, { rule_type: 'entry_criteria', title, description: null, order_position: 0 });
      }
      for (const title of exitRules) {
        await playbookService.createRule(created.id, { rule_type: 'exit_criteria', title, description: null, order_position: 0 });
      }

      onPlaybookCreated?.(created);

      setName('');
      setDescription('');
      setEmoji('');
      setEntryRules([]);
      setExitRules([]);
      onOpenChange(false);
      toast.success('Playbook created successfully');
    } catch {
      toast.error('Failed to create playbook');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleClose = () => {
    if (!isSubmitting) {
      setErrors([]);
      onOpenChange(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-[560px]">
        <DialogHeader>
          <DialogTitle>Create playbook</DialogTitle>
          <DialogDescription>
            Define your setup and add the rules you follow. You can edit these later.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-6">
          <section className="space-y-3">
            <h3 className="text-sm font-semibold">General information</h3>
            <div className="grid gap-3">
              <div className="grid gap-2">
                <Label htmlFor="playbook-name">Playbook name</Label>
                <Input
                  id="playbook-name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="Absorption Reversal"
                  disabled={isSubmitting}
                  required
                />
              </div>

              <div className="grid gap-2">
                <Label htmlFor="emoji-color">Add Icon or choose a colour</Label>
                <div className="flex items-center gap-3">
                  <Popover>
                    <PopoverTrigger asChild>
                      <button
                        type="button"
                        aria-label="Color swatch"
                        className="h-8 w-8 rounded-md border"
                        style={{ backgroundColor: color }}
                        disabled={isSubmitting}
                      />
                    </PopoverTrigger>
                    <PopoverContent className="w-auto p-3" align="start">
                      <HexColorPicker
                        color={colord(color).isValid() ? colord(color).toHex() : '#f59e0b'}
                        onChange={(c) => setColor(colord(c).toHex())}
                      />
                    </PopoverContent>
                  </Popover>
                  <Input
                    value={emoji}
                    onChange={(e) => setEmoji(e.target.value)}
                    placeholder="Optional emoji, e.g. 🔶"
                    className="w-36"
                    disabled={isSubmitting}
                  />
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => setIsEmojiPickerOpen((v) => !v)}
                    disabled={isSubmitting}
                  >
                    {isEmojiPickerOpen ? 'Close emoji' : 'Pick emoji'}
                  </Button>
                </div>
                {isEmojiPickerOpen && (
                  <div className="mt-2">
                    <Picker
                      data={emojiData}
                      onEmojiSelect={(selection: unknown) => {
                        const native = extractNativeEmoji(selection);
                        if (native) {
                          setEmoji(native);
                          setIsEmojiPickerOpen(false);
                        }
                      }}
                      theme="light"
                      previewPosition="none"
                    />
                  </div>
                )}
              </div>

              <div className="grid gap-2">
                <Label htmlFor="label">Label</Label>
                <Input
                  id="label"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder="We're approaching or trading at an area of interest"
                  disabled={isSubmitting}
                />
              </div>
            </div>
          </section>

          <section className="space-y-3">
            <h3 className="text-sm font-semibold">Trading Playbook Rules</h3>
            <p className="text-sm text-muted-foreground">List your rules, group them, and iterate as you learn.</p>

            <div className="space-y-4">
              {/* Entry criteria */}
              <div className="rounded-md border">
                <div className="flex items-center justify-between px-3 py-2">
                  <span className="text-sm font-medium">Entry criteria</span>
                  <Button type="button" variant="ghost" size="sm" onClick={() => addRule('entry')}>Create new rule</Button>
                </div>
                <div className="divide-y">
                  {entryRules.length === 0 ? (
                    <div className="px-3 py-3 text-sm text-muted-foreground">No rules yet</div>
                  ) : (
                    entryRules.map((r, i) => (
                      <div key={i} className="flex items-center justify-between px-3 py-2 text-sm">
                        <span>{r}</span>
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
                              <MoreVertical className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem onClick={() => handleEditRule('entry', i)}>
                              <Edit className="mr-2 h-4 w-4" />
                              Edit
                            </DropdownMenuItem>
                            <DropdownMenuItem onClick={() => removeRule('entry', i)} className="text-destructive">
                              <Trash2 className="mr-2 h-4 w-4" />
                              Delete
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </div>
                    ))
                  )}
                </div>
              </div>

              {/* Exit criteria */}
              <div className="rounded-md border">
                <div className="flex items-center justify-between px-3 py-2">
                  <span className="text-sm font-medium">Exit criteria</span>
                  <Button type="button" variant="ghost" size="sm" onClick={() => addRule('exit')}>Create new rule</Button>
                </div>
                <div className="divide-y">
                  {exitRules.length === 0 ? (
                    <div className="px-3 py-3 text-sm text-muted-foreground">No rules yet</div>
                  ) : (
                    exitRules.map((r, i) => (
                      <div key={i} className="flex items-center justify-between px-3 py-2 text-sm">
                        <span>{r}</span>
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
                              <MoreVertical className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem onClick={() => handleEditRule('exit', i)}>
                              <Edit className="mr-2 h-4 w-4" />
                              Edit
                            </DropdownMenuItem>
                            <DropdownMenuItem onClick={() => removeRule('exit', i)} className="text-destructive">
                              <Trash2 className="mr-2 h-4 w-4" />
                              Delete
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </div>
                    ))
                  )}
                </div>
              </div>
            </div>
          </section>

          {errors.length > 0 && (
            <div className="text-sm text-destructive space-y-1">
              {errors.map((err, idx) => (
                <div key={idx}>{err}</div>
              ))}
            </div>
          )}

          <DialogFooter>
            <Button type="button" variant="outline" onClick={handleClose} disabled={isSubmitting}>Cancel</Button>
            <Button type="submit" disabled={isSubmitting || !canSubmit}>{isSubmitting ? 'Creating...' : 'Create Playbook'}</Button>
          </DialogFooter>
        </form>

        {/* Rule creation dialog */}
        <Dialog open={isRuleDialogOpen} onOpenChange={(v) => setIsRuleDialogOpen(v)}>
          <DialogContent className="sm:max-w-[480px]">
            <DialogHeader>
              <DialogTitle>{editingRuleIndex !== null ? 'Edit' : 'Create new'} {ruleDialogGroup} rule</DialogTitle>
              <DialogDescription>Enter a concise title for your rule.</DialogDescription>
            </DialogHeader>
            <div className="grid gap-2">
              <Label htmlFor="rule-title">Rule title</Label>
              <Input
                id="rule-title"
                value={ruleTitle}
                onChange={(e) => setRuleTitle(e.target.value)}
                placeholder={ruleDialogGroup === 'entry' ? 'e.g. Break of structure with volume' : 'e.g. Target hit or reversal signal'}
                disabled={isSubmitting}
              />
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setIsRuleDialogOpen(false)} disabled={isSubmitting}>Cancel</Button>
              <Button type="button" onClick={confirmAddRule} disabled={isSubmitting || ruleTitle.trim().length === 0}>Save Rule</Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </DialogContent>
    </Dialog>
  );
}
