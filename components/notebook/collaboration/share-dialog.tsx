'use client';

import { useState } from 'react';
import { Copy, Globe, Link2, Lock, Mail, Users, X } from 'lucide-react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Textarea } from '@/components/ui/textarea';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';

import { useCollaboration } from '@/hooks/use-collaboration';
import type { CollaboratorRole, NoteVisibility } from '@/lib/types/collaboration';

interface ShareDialogProps {
  noteId: string;
  noteTitle: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function ShareDialog({ noteId, noteTitle, open, onOpenChange }: ShareDialogProps) {
  const {
    collaborators,
    shareSettings,
    isLoading,
    inviteCollaborator,
    removeCollaborator,
    updateRole,
    updateVisibility,
  } = useCollaboration({ noteId, enabled: open });

  const [inviteEmail, setInviteEmail] = useState('');
  const [inviteRole, setInviteRole] = useState<CollaboratorRole>('editor');
  const [inviteMessage, setInviteMessage] = useState('');
  const [isInviting, setIsInviting] = useState(false);

  const handleInvite = async () => {
    if (!inviteEmail.trim()) {
      toast.error('Please enter an email address');
      return;
    }

    setIsInviting(true);
    try {
      await inviteCollaborator(inviteEmail.trim(), inviteRole, inviteMessage || undefined, undefined, noteTitle);
      toast.success('Invitation sent!');
      setInviteEmail('');
      setInviteMessage('');
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to send invitation');
    } finally {
      setIsInviting(false);
    }
  };

  const handleRemove = async (collaboratorId: string, email: string) => {
    try {
      await removeCollaborator(collaboratorId);
      toast.success(`Removed ${email}`);
    } catch (err) {
      toast.error('Failed to remove collaborator');
    }
  };

  const handleRoleChange = async (collaboratorId: string, role: CollaboratorRole) => {
    try {
      await updateRole(collaboratorId, role);
      toast.success('Role updated');
    } catch (err) {
      toast.error('Failed to update role');
    }
  };

  const handleVisibilityChange = async (visibility: NoteVisibility) => {
    try {
      await updateVisibility(visibility);
      toast.success(`Note is now ${visibility}`);
    } catch (err) {
      toast.error('Failed to update visibility');
    }
  };

  const copyPublicLink = () => {
    if (shareSettings?.public_slug) {
      const url = `${window.location.origin}/public/notes/${shareSettings.public_slug}`;
      navigator.clipboard.writeText(url);
      toast.success('Link copied to clipboard');
    }
  };

  const getInitials = (email: string, name?: string) => {
    if (name) return name.slice(0, 2).toUpperCase();
    return email.slice(0, 2).toUpperCase();
  };

  const getRoleBadgeVariant = (role: CollaboratorRole) => {
    switch (role) {
      case 'owner':
        return 'default';
      case 'editor':
        return 'secondary';
      case 'viewer':
        return 'outline';
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Users className="h-5 w-5" />
            Share "{noteTitle}"
          </DialogTitle>
          <DialogDescription>
            Invite others to collaborate or make this note public.
          </DialogDescription>
        </DialogHeader>

        <Tabs defaultValue="invite" className="mt-4">
          <TabsList className="grid w-full grid-cols-2">
            <TabsTrigger value="invite">
              <Mail className="h-4 w-4 mr-2" />
              Invite
            </TabsTrigger>
            <TabsTrigger value="visibility">
              <Globe className="h-4 w-4 mr-2" />
              Visibility
            </TabsTrigger>
          </TabsList>

          <TabsContent value="invite" className="space-y-4 mt-4">
            {/* Invite Form */}
            <div className="space-y-3">
              <div className="flex gap-2">
                <Input
                  placeholder="Email address"
                  type="email"
                  value={inviteEmail}
                  onChange={(e) => setInviteEmail(e.target.value)}
                  className="flex-1"
                />
                <Select value={inviteRole} onValueChange={(v) => setInviteRole(v as CollaboratorRole)}>
                  <SelectTrigger className="w-[100px]">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="editor">Editor</SelectItem>
                    <SelectItem value="viewer">Viewer</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <Textarea
                placeholder="Add a message (optional)"
                value={inviteMessage}
                onChange={(e) => setInviteMessage(e.target.value)}
                rows={2}
              />
              <Button onClick={handleInvite} disabled={isInviting} className="w-full">
                {isInviting ? 'Sending...' : 'Send Invitation'}
              </Button>
            </div>

            <Separator />

            {/* Collaborators List */}
            <div className="space-y-2">
              <Label className="text-sm font-medium">People with access</Label>
              {isLoading ? (
                <div className="text-sm text-muted-foreground">Loading...</div>
              ) : collaborators.length === 0 ? (
                <div className="text-sm text-muted-foreground">No collaborators yet</div>
              ) : (
                <div className="space-y-2 max-h-[200px] overflow-y-auto">
                  {collaborators.map((collab) => (
                    <div
                      key={collab.id}
                      className="flex items-center justify-between p-2 rounded-lg bg-muted/50"
                    >
                      <div className="flex items-center gap-3">
                        <Avatar className="h-8 w-8">
                          <AvatarFallback className="text-xs">
                            {getInitials(collab.user_email, collab.user_name)}
                          </AvatarFallback>
                        </Avatar>
                        <div>
                          <div className="text-sm font-medium">
                            {collab.user_name || collab.user_email}
                          </div>
                          {collab.user_name && (
                            <div className="text-xs text-muted-foreground">
                              {collab.user_email}
                            </div>
                          )}
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        {collab.role === 'owner' ? (
                          <Badge variant={getRoleBadgeVariant(collab.role)}>Owner</Badge>
                        ) : (
                          <>
                            <Select
                              value={collab.role}
                              onValueChange={(v) => handleRoleChange(collab.id, v as CollaboratorRole)}
                            >
                              <SelectTrigger className="h-7 w-[90px] text-xs">
                                <SelectValue />
                              </SelectTrigger>
                              <SelectContent>
                                <SelectItem value="editor">Editor</SelectItem>
                                <SelectItem value="viewer">Viewer</SelectItem>
                              </SelectContent>
                            </Select>
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-7 w-7"
                              onClick={() => handleRemove(collab.id, collab.user_email)}
                            >
                              <X className="h-4 w-4" />
                            </Button>
                          </>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </TabsContent>

          <TabsContent value="visibility" className="space-y-4 mt-4">
            <div className="space-y-3">
              <Label className="text-sm font-medium">Who can access this note?</Label>
              
              <div className="space-y-2">
                <button
                  onClick={() => handleVisibilityChange('private')}
                  className={`w-full p-3 rounded-lg border text-left transition-colors ${
                    shareSettings?.visibility === 'private' || !shareSettings
                      ? 'border-primary bg-primary/5'
                      : 'border-border hover:bg-muted/50'
                  }`}
                >
                  <div className="flex items-center gap-3">
                    <Lock className="h-5 w-5" />
                    <div>
                      <div className="font-medium">Private</div>
                      <div className="text-xs text-muted-foreground">Only you can access</div>
                    </div>
                  </div>
                </button>

                <button
                  onClick={() => handleVisibilityChange('shared')}
                  className={`w-full p-3 rounded-lg border text-left transition-colors ${
                    shareSettings?.visibility === 'shared'
                      ? 'border-primary bg-primary/5'
                      : 'border-border hover:bg-muted/50'
                  }`}
                >
                  <div className="flex items-center gap-3">
                    <Users className="h-5 w-5" />
                    <div>
                      <div className="font-medium">Shared</div>
                      <div className="text-xs text-muted-foreground">
                        Only invited collaborators can access
                      </div>
                    </div>
                  </div>
                </button>

                <button
                  onClick={() => handleVisibilityChange('public')}
                  className={`w-full p-3 rounded-lg border text-left transition-colors ${
                    shareSettings?.visibility === 'public'
                      ? 'border-primary bg-primary/5'
                      : 'border-border hover:bg-muted/50'
                  }`}
                >
                  <div className="flex items-center gap-3">
                    <Globe className="h-5 w-5" />
                    <div>
                      <div className="font-medium">Public</div>
                      <div className="text-xs text-muted-foreground">
                        Anyone with the link can view (read-only)
                      </div>
                    </div>
                  </div>
                </button>
              </div>

              {shareSettings?.visibility === 'public' && shareSettings.public_slug && (
                <div className="mt-4 p-3 rounded-lg bg-muted">
                  <Label className="text-xs text-muted-foreground">Public link</Label>
                  <div className="flex items-center gap-2 mt-1">
                    <Input
                      readOnly
                      value={`${window.location.origin}/public/notes/${shareSettings.public_slug}`}
                      className="text-xs"
                    />
                    <Button variant="outline" size="icon" onClick={copyPublicLink}>
                      <Copy className="h-4 w-4" />
                    </Button>
                  </div>
                  <div className="text-xs text-muted-foreground mt-2">
                    {shareSettings.view_count} views
                  </div>
                </div>
              )}
            </div>
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
