"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { useAIChat } from "@/hooks/use-ai-chat";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import {
  Loader2,
  MessageCircle,
  Plus,
  Calendar,
  MessageSquare,
} from "lucide-react";
import Link from "next/link";
import { AppPageHeader } from "@/components/app-page-header";

export default function ChatListPage() {
  const router = useRouter();
  const {
    sessions,
    isLoading,
    error,
    loadSessions,
  } = useAIChat();

  useEffect(() => {
    // Load sessions when component mounts
    loadSessions();
  }, [loadSessions]);

  const handleNewChat = () => {
    router.push("/app/chat/new");
  };

  if (isLoading && !sessions.length) {
    return (
      <div className="h-screen flex items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-screen flex items-center justify-center">
        <Card className="w-96">
          <CardHeader>
            <CardTitle className="text-red-600">Error</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground mb-4">
              {error.message}
            </p>
            <Button onClick={() => loadSessions()} variant="outline">
              Try Again
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  const headerActions = (
    <Button onClick={handleNewChat}>
      <Plus className="h-4 w-4 mr-2" />
      New Chat
    </Button>
  );

  return (
    <div className="h-screen flex flex-col">
      <AppPageHeader title="AI Chat Sessions" actions={headerActions} />

      {/* Chat Sessions List */}
      <div className="flex-1 overflow-hidden">
        <ScrollArea className="h-full">
          <div className="p-6">
            {sessions.length === 0 ? (
              <div className="text-center py-12">
                <MessageCircle className="h-16 w-16 mx-auto mb-4 opacity-50" />
                <h3 className="text-lg font-semibold mb-2">
                  No chat sessions yet
                </h3>
                <p className="text-muted-foreground mb-6">
                  Start your first conversation with AI about your trading data
                  and strategies
                </p>
                <Button onClick={handleNewChat}>
                  <Plus className="h-4 w-4 mr-2" />
                  Start New Chat
                </Button>
              </div>
            ) : (
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                {sessions.map((session) => (
                  <Link key={session.id} href={`/app/chat/${session.id}`}>
                    <Card className="hover:shadow-md transition-shadow cursor-pointer">
                      <CardHeader className="pb-3">
                        <CardTitle className="text-lg flex items-center justify-between">
                          <span className="truncate">
                            {session.title || "Untitled Chat"}
                          </span>
                          <Badge variant="secondary" className="ml-2">
                            {session.message_count || 0}
                          </Badge>
                        </CardTitle>
                      </CardHeader>
                      <CardContent className="pt-0">
                        <div className="flex items-center justify-between text-sm text-muted-foreground">
                          <div className="flex items-center gap-1">
                            <Calendar className="h-4 w-4" />
                            <span>
                              {new Date(
                                session.created_at,
                              ).toLocaleDateString()}
                            </span>
                          </div>
                          <div className="flex items-center gap-1">
                            <MessageSquare className="h-4 w-4" />
                            <span>
                              {new Date(
                                session.created_at,
                              ).toLocaleTimeString()}
                            </span>
                          </div>
                        </div>
                        <div className="mt-3">
                          <Badge variant="outline" className="text-xs">
                            Last updated{" "}
                            {new Date(
                              session.updated_at || session.created_at,
                            ).toLocaleDateString()}
                          </Badge>
                        </div>
                      </CardContent>
                    </Card>
                  </Link>
                ))}
              </div>
            )}
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}
