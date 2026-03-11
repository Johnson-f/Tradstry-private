'use client';

import { useState } from 'react';
import { Check, X, Mail } from 'lucide-react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { useInvitations } from '@/hooks/use-collaboration';

export function InvitationBanner() {
  const { invitations, acceptInvitation, declineInvitation, isLoading } = useInvitations();
  const [processingId, setProcessingId] = useState<string | null>(null);

  if (isLoading || invitations.length === 0) return null;

  const handleAccept = async (invitation: typeof invitations[0]) => {
    setProcessingId(invitation.id);
    try {
      await acceptInvitation(invitation.token);
      toast.success('Invitation accepted! You can now access the note.');
    } catch (err) {
      toast.error('Failed to accept invitation');
    } finally {
      setProcessingId(null);
    }
  };

  const handleDecline = async (invitation: typeof invitations[0]) => {
    setProcessingId(invitation.id);
    try {
      await declineInvitation(invitation.id);
      toast.success('Invitation declined');
    } catch (err) {
      toast.error('Failed to decline invitation');
    } finally {
      setProcessingId(null);
    }
  };

  return (
    <div className="space-y-2 mb-4">
      {invitations.map((invitation) => (
        <Card key={invitation.id} className="p-3">
          <div className="flex items-center justify-between gap-4">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-full bg-primary/10">
                <Mail className="h-4 w-4 text-primary" />
              </div>
              <div>
                <div className="text-sm font-medium">
                  {invitation.inviter_email} invited you to collaborate
                </div>
                <div className="text-xs text-muted-foreground">
                  Role: <span className="capitalize">{invitation.role}</span>
                  {invitation.message && ` • "${invitation.message}"`}
                </div>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                variant="outline"
                onClick={() => handleDecline(invitation)}
                disabled={processingId === invitation.id}
              >
                <X className="h-4 w-4 mr-1" />
                Decline
              </Button>
              <Button
                size="sm"
                onClick={() => handleAccept(invitation)}
                disabled={processingId === invitation.id}
              >
                <Check className="h-4 w-4 mr-1" />
                Accept
              </Button>
            </div>
          </div>
        </Card>
      ))}
    </div>
  );
}
