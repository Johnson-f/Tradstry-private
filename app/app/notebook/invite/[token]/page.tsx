'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { useAuth } from '@/lib/hooks/use-auth';
import { collaborationService } from '@/lib/services/collaboration-service';
import { Loader2, CheckCircle, XCircle, Mail } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { toast } from 'sonner';

interface PageProps {
  params: Promise<{ token: string }>;
}

export default function InvitePage({ params }: PageProps) {
  const router = useRouter();
  const { user, loading: authLoading } = useAuth();
  const [token, setToken] = useState<string | null>(null);
  const [status, setStatus] = useState<'loading' | 'ready' | 'accepting' | 'success' | 'error'>('loading');
  const [error, setError] = useState<string | null>(null);
  const [noteId, setNoteId] = useState<string | null>(null);

  useEffect(() => {
    params.then(p => setToken(p.token));
  }, [params]);

  useEffect(() => {
    if (token && !authLoading) {
      if (!user) {
        // Redirect to login with return URL
        const returnUrl = encodeURIComponent(`/app/notebook/invite/${token}`);
        router.push(`/auth/login?returnUrl=${returnUrl}`);
      } else {
        setStatus('ready');
      }
    }
  }, [token, user, authLoading, router]);

  const handleAccept = async () => {
    if (!token || !user) return;
    
    setStatus('accepting');
    try {
      const result = await collaborationService.acceptInvitation({
        token,
        user_name: user.user_metadata?.full_name,
      });
      setNoteId(result.note_id);
      setStatus('success');
      toast.success('Invitation accepted!');
      // Redirect to the note after a short delay
      setTimeout(() => {
        router.push(`/app/notebook?note=${result.note_id}`);
      }, 1500);
    } catch (err) {
      setStatus('error');
      setError(err instanceof Error ? err.message : 'Failed to accept invitation');
      toast.error('Failed to accept invitation');
    }
  };

  const handleDecline = () => {
    router.push('/app/notebook');
  };

  if (authLoading || status === 'loading') {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-primary" />
      </div>
    );
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900 p-4">
      <Card className="w-full max-w-md">
        <CardHeader className="text-center">
          {status === 'success' ? (
            <CheckCircle className="h-12 w-12 text-green-500 mx-auto mb-4" />
          ) : status === 'error' ? (
            <XCircle className="h-12 w-12 text-red-500 mx-auto mb-4" />
          ) : (
            <Mail className="h-12 w-12 text-primary mx-auto mb-4" />
          )}
          <CardTitle>
            {status === 'success' ? 'Invitation Accepted!' : 
             status === 'error' ? 'Invitation Failed' : 
             'Collaboration Invitation'}
          </CardTitle>
          <CardDescription>
            {status === 'success' ? 'Redirecting you to the note...' :
             status === 'error' ? error :
             'You have been invited to collaborate on a note'}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {status === 'ready' && (
            <>
              <p className="text-sm text-muted-foreground text-center">
                Click accept to join as a collaborator and start editing together.
              </p>
              <div className="flex gap-3">
                <Button variant="outline" className="flex-1" onClick={handleDecline}>
                  Decline
                </Button>
                <Button className="flex-1" onClick={handleAccept}>
                  Accept Invitation
                </Button>
              </div>
            </>
          )}
          {status === 'accepting' && (
            <div className="flex justify-center">
              <Loader2 className="h-6 w-6 animate-spin" />
            </div>
          )}
          {status === 'error' && (
            <Button className="w-full" onClick={() => router.push('/app/notebook')}>
              Go to Notebook
            </Button>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
