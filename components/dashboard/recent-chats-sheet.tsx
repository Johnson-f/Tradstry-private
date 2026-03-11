"use client";

import { useState } from "react";
import Link from "next/link";
import { useAIChat } from "@/hooks/use-ai-chat";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Loader2, MessageCircle, Calendar, MessageSquare, MoreHorizontal, Trash2 } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { toast } from "sonner";

interface RecentChatsSheetProps {
  children: React.ReactNode;
}

export default function RecentChatsSheet({ children }: RecentChatsSheetProps) {
  const [open, setOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [sessionToDelete, setSessionToDelete] = useState<{ id: string; title: string } | null>(null);
  const {
    sessions,
    isLoading: loading,
    error,
    deleteSession,
    loadSessions: refetchSessions,
  } = useAIChat();

  const clearError = () => {
    refetchSessions();
  };

  const handleDeleteClick = (e: React.MouseEvent, session: { id: string; title: string }) => {
    e.preventDefault();
    e.stopPropagation();
    setSessionToDelete(session);
    setDeleteDialogOpen(true);
  };

  const handleDeleteConfirm = async () => {
    if (!sessionToDelete) return;

    try {
      await deleteSession(sessionToDelete.id);
      toast.success("Chat session deleted successfully");
      setDeleteDialogOpen(false);
      setSessionToDelete(null);
    } catch (error) {
      toast.error("Failed to delete chat session");
      console.error("Delete error:", error);
    }
  };

  const handleDeleteCancel = () => {
    setDeleteDialogOpen(false);
    setSessionToDelete(null);
  };

  return (
    <Sheet open={open} onOpenChange={setOpen}>
      <SheetTrigger asChild>
        {children}
      </SheetTrigger>
      <SheetContent side="right" className="w-[360px] sm:w-[460px]">
        <SheetHeader className="pb-4">
          <div className="flex items-center justify-between">
            <SheetTitle className="flex items-center gap-2">
              <MessageCircle className="h-5 w-5" />
              Recents
            </SheetTitle>
          </div>
        </SheetHeader>
        
        <ScrollArea className="h-[calc(100vh-100px)] pr-4">
          <div className="space-y-3">
            {loading && sessions.length === 0 ? (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="h-8 w-8 animate-spin" />
              </div>
            ) : error ? (
              <div className="p-6 text-center">
                <p className="text-red-600 text-sm mb-4">
                  {error instanceof Error ? error.message : String(error)}
                </p>
                <Button onClick={clearError} variant="outline" size="sm">
                  Try Again
                </Button>
              </div>
            ) : sessions.length === 0 ? (
              <div className="p-6 text-center">
                <MessageCircle className="h-12 w-12 mx-auto mb-4 opacity-50" />
                <h4 className="font-semibold mb-2">No conversations yet</h4>
                <p className="text-sm text-muted-foreground mb-4">
                  Start your first conversation with AI
                </p>
              </div>
            ) : (
              sessions.map((session) => (
                <div key={session.id} className="group p-4 rounded-lg bg-card hover:bg-accent/50 border transition-colors">
                  <Link 
                    href={`/app/chat/${session.id}`}
                    onClick={() => setOpen(false)}
                    className="block"
                  >
                    <div className="flex items-start justify-between gap-2 mb-2">
                      <h4 className="font-medium text-sm leading-tight flex-1 min-w-0 pr-2">
                        {session.title || "Untitled Chat"}
                      </h4>
                      <div className="flex items-center gap-1 flex-shrink-0" onClick={(e) => e.preventDefault()}>
                        <Badge variant="default" className="text-xs px-2 py-0.5">
                          {session.message_count || 0}
                        </Badge>
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button 
                              variant="ghost" 
                              size="sm" 
                              className="h-6 w-6 p-0"
                            >
                              <MoreHorizontal className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem 
                              onClick={(e) => handleDeleteClick(e, { id: session.id, title: session.title || "Untitled Chat" })}
                              className="text-red-600 focus:text-red-600"
                            >
                              <Trash2 className="h-4 w-4 mr-2" />
                              Delete
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </div>
                    </div>
                    
                    <div className="flex items-center gap-4 text-xs text-muted-foreground">
                      <div className="flex items-center gap-1">
                        <Calendar className="h-3 w-3" />
                        <span>{new Date(session.created_at).toLocaleDateString()}</span>
                      </div>
                      <div className="flex items-center gap-1">
                        <MessageSquare className="h-3 w-3" />
                        <span>{new Date(session.created_at).toLocaleTimeString([], { 
                          hour: '2-digit', 
                          minute: '2-digit' 
                        })}</span>
                      </div>
                    </div>
                    
                    {session.updated_at && session.updated_at !== session.created_at && (
                      <div className="mt-2">
                        <Badge variant="outline" className="text-xs">
                          Updated {new Date(session.updated_at).toLocaleDateString()}
                        </Badge>
                      </div>
                    )}
                  </Link>
                </div>
              ))
            )}
          </div>
        </ScrollArea>
      </SheetContent>
      
      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Chat Session</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete &quot;{sessionToDelete?.title || 'Untitled Chat'}&quot;? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={handleDeleteCancel}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={handleDeleteConfirm}>
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Sheet>
  );
}
