 /**
 * Playbook Edit Dialog Component
 */

import { useState, useEffect } from 'react';
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
import { Textarea } from '@/components/ui/textarea';
import { usePlaybooks, usePlaybookRules } from '@/lib/hooks/use-playbooks';
import { toast } from 'sonner';
import type { Playbook, PlaybookRule } from '@/lib/types/playbook';
import Picker from '@emoji-mart/react';
import emojiData from '@emoji-mart/data';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { HexColorPicker } from 'react-colorful';
import { colord } from 'colord';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { MoreVertical, Edit, Trash2 } from 'lucide-react';

type PlaybookFormData = { name: string; description?: string };
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

interface PlaybookEditDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  playbook: Playbook | null;
  onPlaybookUpdated: (playbook: Playbook) => void;
}

export function PlaybookEditDialog({
  open,
  onOpenChange,
  playbook,
  onPlaybookUpdated,
}: PlaybookEditDialogProps) {
  const { updatePlaybook } = usePlaybooks();
  const { data: existingRules = [], createRule, deleteRule } = usePlaybookRules(playbook?.id || '');
  
  const [formData, setFormData] = useState<PlaybookFormData>({
    name: '',
    description: '',
  });
  const [color, setColor] = useState<string>('#f59e0b');
  const [emoji, setEmoji] = useState<string>('');

  // Emoji picker
  const [isEmojiPickerOpen, setIsEmojiPickerOpen] = useState(false);
  const [isRuleDialogOpen, setIsRuleDialogOpen] = useState(false);
  const [ruleDialogGroup, setRuleDialogGroup] = useState<RuleGroupKey>('entry');
  const [ruleTitle, setRuleTitle] = useState('');
  const [editingRuleId, setEditingRuleId] = useState<string | null>(null);

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [errors, setErrors] = useState<string[]>([]);

  // Inline utility function
  const validatePlaybookData = (data: PlaybookFormData): { isValid: boolean; errors: string[] } => {
    const errors: string[] = [];

    if (!data.name || data.name.trim().length === 0) {
      errors.push('Playbook name is required');
    }

    if (data.name && data.name.length > 100) {
      errors.push('Playbook name must be less than 100 characters');
    }

    if (data.description && data.description.length > 500) {
      errors.push('Description must be less than 500 characters');
    }

    return {
      isValid: errors.length === 0,
      errors
    };
  };

  // Update form data when playbook changes
  useEffect(() => {
    if (playbook) {
      setFormData({
        name: playbook.name,
        description: playbook.description || '',
      });
      setColor(playbook.color || '#f59e0b');
      setEmoji(playbook.emoji || '');
    }
  }, [playbook]);

  const addRule = (group: RuleGroupKey) => {
    setRuleDialogGroup(group);
    setRuleTitle('');
    setIsRuleDialogOpen(true);
  };

  const confirmAddRule = async () => {
    const trimmed = ruleTitle.trim();
    if (!trimmed) return;
    if (playbook) {
      const ruleType = ruleDialogGroup === 'entry' ? 'entry_criteria' : 'exit_criteria';
      if (editingRuleId) {
        await deleteRule(editingRuleId);
      }
      await createRule({ rule_type: ruleType, title: trimmed, description: null, order_position: 0 });
    }
    setIsRuleDialogOpen(false);
    setRuleTitle('');
    setEditingRuleId(null);
  };

  const handleEditRule = (rule: PlaybookRule) => {
    setRuleDialogGroup(rule.rule_type === 'entry_criteria' ? 'entry' : 'exit');
    setRuleTitle(rule.title);
    setEditingRuleId(rule.id);
    setIsRuleDialogOpen(true);
  };

  const handleDeleteRule = async (ruleId: string) => {
    if (playbook) {
      await deleteRule(ruleId);
    }
  };

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>): Promise<void> => {
    e.preventDefault();
    if (!playbook) return;
    
    const validation = validatePlaybookData(formData);
    if (!validation.isValid) {
      setErrors(validation.errors);
      return;
    }

    setIsSubmitting(true);
    setErrors([]);

    try {
      const updatedPlaybook = (await updatePlaybook(
        playbook.id,
        {
          name: formData.name,
          description: formData.description || null,
          color: color || null,
          emoji: emoji || null,
        }
      )) as Playbook;
      onPlaybookUpdated(updatedPlaybook);
      onOpenChange(false);
      toast.success('Playbook updated successfully');
    } catch (error) {
      console.error('Error updating playbook:', error);
      toast.error('Failed to update playbook');
      setErrors(['Failed to update playbook. Please try again.']);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleInputChange = (field: keyof PlaybookFormData, value: string): void => {
    setFormData(prev => ({ ...prev, [field]: value }));
    if (errors.length > 0) {
      setErrors([]);
    }
  };

  const handleClose = (): void => {
    if (!isSubmitting) {
      setErrors([]);
      onOpenChange(false);
    }
  };

  if (!playbook) {
    return null;
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(nextOpen: boolean) => {
        if (nextOpen) {
          onOpenChange(true);
        } else {
          handleClose();
        }
      }}
    >
      <DialogContent className="sm:max-w-[560px]">
        <DialogHeader>
          <DialogTitle>Edit Playbook</DialogTitle>
          <DialogDescription>
            Update the details of your trading setup playbook.
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
                value={formData.name}
                onChange={(e) => handleInputChange('name', e.target.value)}
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
              <Label htmlFor="description">Description</Label>
              <Textarea
                id="description"
                value={formData.description || ''}
                onChange={(e) => handleInputChange('description', e.target.value)}
                placeholder="Describe this trading setup..."
                disabled={isSubmitting}
                rows={3}
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
                  {existingRules.filter((r) => r.rule_type === 'entry_criteria').length === 0 ? (
                    <div className="px-3 py-3 text-sm text-muted-foreground">No rules yet</div>
                  ) : (
                    existingRules.filter((r) => r.rule_type === 'entry_criteria').map((rule) => (
                      <div key={rule.id} className="flex items-center justify-between px-3 py-2 text-sm">
                        <span>{rule.title}</span>
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
                              <MoreVertical className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem onClick={() => handleEditRule(rule)}>
                              <Edit className="mr-2 h-4 w-4" />
                              Edit
                            </DropdownMenuItem>
                            <DropdownMenuItem onClick={() => handleDeleteRule(rule.id)} className="text-destructive">
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
                  {existingRules.filter((r) => r.rule_type === 'exit_criteria').length === 0 ? (
                    <div className="px-3 py-3 text-sm text-muted-foreground">No rules yet</div>
                  ) : (
                    existingRules.filter((r) => r.rule_type === 'exit_criteria').map((rule) => (
                      <div key={rule.id} className="flex items-center justify-between px-3 py-2 text-sm">
                        <span>{rule.title}</span>
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
                              <MoreVertical className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem onClick={() => handleEditRule(rule)}>
                              <Edit className="mr-2 h-4 w-4" />
                              Edit
                            </DropdownMenuItem>
                            <DropdownMenuItem onClick={() => handleDeleteRule(rule.id)} className="text-destructive">
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
              <div className="text-sm text-destructive">
                {errors.map((error, index) => (
                  <div key={index}>{error}</div>
                ))}
              </div>
            )}
          
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={handleClose}
              disabled={isSubmitting}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? 'Updating...' : 'Update Playbook'}
            </Button>
          </DialogFooter>
        </form>

        {/* Rule creation dialog */}
        <Dialog open={isRuleDialogOpen} onOpenChange={(v) => setIsRuleDialogOpen(v)}>
          <DialogContent className="sm:max-w-[480px]">
            <DialogHeader>
              <DialogTitle>Create new {ruleDialogGroup} rule</DialogTitle>
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
