/**
 * Playbook Delete Dialog Component
 */

import { useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

import { usePlaybooks } from '@/lib/hooks/use-playbooks';
import { toast } from 'sonner';
import { AlertTriangle } from 'lucide-react';
import type { Playbook } from '@/lib/types/playbook';

function formatDateSafe(value?: string | null): string {
  const raw = value || '';
  const d = new Date(raw);
  return isNaN(d.getTime()) ? '' : d.toLocaleDateString();
}

function getCreatedAt(pb: Playbook): string | null {
  const snake = (pb as unknown as { created_at?: string }).created_at;
  if (typeof snake === 'string') return snake;
  const camel = (pb as unknown as { createdAt?: string }).createdAt;
  return typeof camel === 'string' ? camel : null;
}

interface PlaybookDeleteDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  playbook: Playbook | null;
  onPlaybookDeleted: (playbookId: string) => void;
}

export function PlaybookDeleteDialog({
  open,
  onOpenChange,
  playbook,
  onPlaybookDeleted,
}: PlaybookDeleteDialogProps) {
  const { deletePlaybook } = usePlaybooks();
  
  const [isDeleting, setIsDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleDelete = async () => {
    if (!playbook) return;
    
    setIsDeleting(true);
    setError(null);

    try {
      await deletePlaybook(playbook.id);
      onPlaybookDeleted(playbook.id);
      onOpenChange(false);
      toast.success('Playbook deleted successfully');
    } catch (error) {
      console.error('Error deleting playbook:', error);
      toast.error('Failed to delete playbook');
      setError('Failed to delete playbook. Please try again.');
    } finally {
      setIsDeleting(false);
    }
  };

  const handleClose = () => {
    if (!isDeleting) {
      setError(null);
      onOpenChange(false);
    }
  };

  if (!playbook) {
    return null;
  }

  const created = formatDateSafe(getCreatedAt(playbook));

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <AlertTriangle className="h-5 w-5 text-destructive" />
            Delete Playbook
          </DialogTitle>
          <DialogDescription>
            Are you sure you want to delete &quot;{playbook.name}&quot;? This action cannot be undone.
            Any trades associated with this playbook will have their associations removed.
          </DialogDescription>
        </DialogHeader>
        
        <div className="py-4">
          <div className="bg-muted p-4 rounded-lg">
            <h4 className="font-medium mb-2">Playbook Details:</h4>
            <div className="flex items-center gap-3 mb-2">
              <div
                className="h-10 w-10 rounded-full border flex items-center justify-center text-xl"
                style={{ backgroundColor: playbook.color || 'var(--muted)' }}
                aria-label="Playbook icon"
              >
                {playbook.emoji || ''}
              </div>
              <p className="text-sm text-muted-foreground">
                <strong>Name:</strong> {playbook.name}
              </p>
            </div>
            {playbook.description && (
              <p className="text-sm text-muted-foreground">
                <strong>Description:</strong> {playbook.description}
              </p>
            )}
            <p className="text-sm text-muted-foreground">
              <strong>Created:</strong> {created || 'Unavailable'}
            </p>
          </div>
          
          {error && (
            <div className="mt-4 text-sm text-destructive">
              {error}
            </div>
          )}
        </div>
        
        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={handleClose}
            disabled={isDeleting}
          >
            Cancel
          </Button>
          <Button
            type="button"
            variant="destructive"
            onClick={handleDelete}
            disabled={isDeleting}
          >
            {isDeleting ? 'Deleting...' : 'Delete Playbook'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
