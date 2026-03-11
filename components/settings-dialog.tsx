"use client"

import * as React from "react"
import { useRouter } from "next/navigation"
import { useTheme } from "next-themes"
import { createClient } from "@/lib/supabase/client"
import {
  Bell,
  Settings,
  User,
  Trash2,
  Sun,
  Moon,
  Laptop,
  Edit2,
  Save,
  X,
  Link as LinkIcon,
  CheckCircle2,
  Clock,
  XCircle,
  Plus,
  RefreshCw,
  Activity,
  Loader2,
  ArrowRight,
} from "lucide-react"

import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "@/components/ui/breadcrumb"
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog"
import { Progress } from "@/components/ui/progress"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { useBrokerage } from "@/lib/hooks/use-brokerage"
import { toast } from "sonner"
import type { ConnectBrokerageRequest } from "@/lib/types/brokerage"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
} from "@/components/ui/sidebar"
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { NotificationsPreferences } from "@/components/notifications"

const data = {
  nav: [
    { name: "General", icon: Settings },
    { name: "Notifications", icon: Bell },
    { name: "Brokerage", icon: LinkIcon },
    { name: "Account", icon: User },
  ],
}

interface SettingsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

interface AccountSettingsContentProps {
  onOpenChange: (open: boolean) => void
}

interface UserProfileData {
  nickname: string | null;
  display_name: string | null;
  timezone: string | null;
  currency: string | null;
  trading_experience_level: string | null;
  primary_trading_goal: string | null;
  asset_types: string | null;
  trading_style: string | null;
  profile_picture_uuid: string | null;
}

interface ProfileApiResponse {
  success: boolean;
  profile?: UserProfileData;
  error?: string;
}

function AccountSettingsContent({ onOpenChange }: AccountSettingsContentProps) {
  const router = useRouter()
  const [isDeleting, setIsDeleting] = React.useState(false)
  const [isEditing, setIsEditing] = React.useState(false)
  const [storageUsage, setStorageUsage] = React.useState<{
    used_mb: number;
    limit_mb: number;
    percentage_used: number;
  } | null>(null)
  const [isLoadingStorage, setIsLoadingStorage] = React.useState(true)
  const [profile, setProfile] = React.useState<UserProfileData | null>(null)
  const [isLoadingProfile, setIsLoadingProfile] = React.useState(true)
  const [isSaving, setIsSaving] = React.useState(false)
  const [userEmail, setUserEmail] = React.useState<string>("")
  const [editableFields, setEditableFields] = React.useState({
    nickname: "",
    display_name: "",
  })

  // Fetch user email and profile on mount
  React.useEffect(() => {
    const fetchUserData = async () => {
      try {
        const supabase = createClient();
        const { data: { user: authUser }, error: authError } = await supabase.auth.getUser();
        
        if (authError || !authUser) {
          console.error('Failed to get auth user:', authError);
          setIsLoadingProfile(false);
          return;
        }

        setUserEmail(authUser.email || "");

        // Fetch profile from backend
        const profileResponse = await fetch(`/api/user/profile/${authUser.id}`, {
          method: 'GET',
          headers: {
            'Content-Type': 'application/json',
          },
        });

        if (profileResponse.ok) {
          const result: ProfileApiResponse = await profileResponse.json();
          if (result.success && result.profile) {
            setProfile(result.profile);
            setEditableFields({
              nickname: result.profile.nickname || "",
              display_name: result.profile.display_name || "",
            });
          }
        }
      } catch (error) {
        console.error('Failed to fetch profile:', error);
      } finally {
        setIsLoadingProfile(false);
      }
    };

    fetchUserData();
  }, []);

  // Fetch storage usage on mount
  React.useEffect(() => {
    const fetchStorageUsage = async () => {
      try {
        const { accountService } = await import('@/lib/services/account-service');
        const usage = await accountService.getStorageUsage();
        setStorageUsage({
          used_mb: usage.used_mb,
          limit_mb: usage.limit_mb,
          percentage_used: usage.percentage_used,
        });
      } catch (error) {
        console.error('Failed to fetch storage usage:', error);
      } finally {
        setIsLoadingStorage(false);
      }
    };

    fetchStorageUsage();
  }, []);

  const handleSaveProfile = async () => {
    setIsSaving(true);
    try {
      const supabase = createClient();
      const { data: { user: authUser } } = await supabase.auth.getUser();
      
      if (!authUser) {
        throw new Error('No authenticated user');
      }

      const response = await fetch(`/api/user/profile/${authUser.id}`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          nickname: editableFields.nickname || null,
          display_name: editableFields.display_name || null,
        }),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: 'Failed to update profile' }));
        throw new Error(errorData.error || 'Failed to update profile');
      }

      const result = await response.json();
      if (result.success) {
        setProfile({
          ...profile,
          nickname: editableFields.nickname || null,
          display_name: editableFields.display_name || null,
        } as UserProfileData);
        setIsEditing(false);
      } else {
        throw new Error(result.error || 'Failed to update profile');
      }
    } catch (error) {
      console.error('Failed to save profile:', error);
      alert('Failed to update profile. Please try again.');
    } finally {
      setIsSaving(false);
    }
  };

  const handleCancelEdit = () => {
    if (profile) {
      setEditableFields({
        nickname: profile.nickname || "",
        display_name: profile.display_name || "",
      });
    }
    setIsEditing(false);
  };

  const handleDeleteAccount = async () => {
    const confirmed = window.confirm(
      '⚠️ WARNING: This will permanently delete your account and ALL your data.\n\n' +
      'This action cannot be undone. Are you absolutely sure?'
    );
    
    if (!confirmed) return;

    const typed = window.prompt('Type "DELETE" (all caps) to confirm account deletion:');

    if (typed !== 'DELETE') {
      alert('Account deletion cancelled. You must type "DELETE" exactly to confirm.');
      return;
    }

    setIsDeleting(true);
    try {
      const { accountService } = await import('@/lib/services/account-service');
      await accountService.deleteAccount();
      onOpenChange(false);
      router.push('/auth/login');
    } catch (error) {
      setIsDeleting(false);
      alert('Failed to delete account. Please try again or contact support.');
      console.error('Account deletion error:', error);
    }
  }

  return (
    <div className="space-y-4">
      {/* Account Information Section */}
      <div className="rounded-lg border bg-card p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-base font-semibold">Account Information</h3>
          {!isEditing && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setIsEditing(true)}
              className="h-8"
            >
              <Edit2 className="h-4 w-4 mr-2" />
              Edit
            </Button>
          )}
        </div>
        <div className="space-y-4">
          {isLoadingProfile ? (
            <div className="space-y-3">
              <div className="h-10 bg-muted rounded animate-pulse" />
              <div className="h-10 bg-muted rounded animate-pulse" />
            </div>
          ) : (
            <>
              <div className="space-y-2">
                <Label htmlFor="email">Email</Label>
                <Input
                  id="email"
                  type="email"
                  value={userEmail}
                  disabled
                  className="bg-muted"
                />
                <p className="text-xs text-muted-foreground">
                  Email cannot be changed from here
                </p>
              </div>
              <div className="space-y-2">
                <Label htmlFor="nickname">Nickname</Label>
                <Input
                  id="nickname"
                  type="text"
                  value={editableFields.nickname}
                  onChange={(e) => setEditableFields({ ...editableFields, nickname: e.target.value })}
                  disabled={!isEditing}
                  placeholder="Enter your nickname"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="display_name">Display Name</Label>
                <Input
                  id="display_name"
                  type="text"
                  value={editableFields.display_name}
                  onChange={(e) => setEditableFields({ ...editableFields, display_name: e.target.value })}
                  disabled={!isEditing}
                  placeholder="Enter your display name"
                />
              </div>
              {isEditing && (
                <div className="flex gap-2 pt-2">
                  <Button
                    onClick={handleSaveProfile}
                    disabled={isSaving}
                    size="sm"
                  >
                    <Save className="h-4 w-4 mr-2" />
                    {isSaving ? 'Saving...' : 'Save Changes'}
                  </Button>
                  <Button
                    variant="outline"
                    onClick={handleCancelEdit}
                    disabled={isSaving}
                    size="sm"
                  >
                    <X className="h-4 w-4 mr-2" />
                    Cancel
                  </Button>
                </div>
              )}
            </>
          )}
        </div>
      </div>

      {/* Storage Usage Section */}
      <div className="rounded-lg border bg-card p-6">
        <h3 className="text-base font-semibold mb-4">Storage Usage</h3>
        <div className="space-y-4">
          {isLoadingStorage ? (
            <div className="space-y-2">
              <div className="h-2 bg-muted rounded animate-pulse" />
              <div className="h-4 bg-muted rounded animate-pulse w-1/3" />
            </div>
          ) : storageUsage ? (
            <>
              <div className="space-y-2">
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Storage Used</span>
                  <span className="font-medium">
                    {storageUsage.used_mb.toFixed(2)} MB / {storageUsage.limit_mb.toFixed(0)} MB
                  </span>
                </div>
                <Progress value={storageUsage.percentage_used} className="h-2" />
                <p className="text-xs text-muted-foreground">
                  {storageUsage.percentage_used.toFixed(1)}% of your storage limit used
                </p>
              </div>
            </>
          ) : (
            <p className="text-sm text-muted-foreground">
              Unable to load storage usage information.
            </p>
          )}
        </div>
      </div>

      {/* Danger Zone Section */}
      <div className="rounded-lg border bg-card p-6">
        <h3 className="text-base font-semibold mb-4">Danger Zone</h3>
        <div className="space-y-4">
          <div>
            <p className="text-sm text-muted-foreground mb-4">
              Once you delete your account, there is no going back. Please be certain.
            </p>
            <button
              onClick={handleDeleteAccount}
              disabled={isDeleting}
              className="inline-flex items-center justify-center rounded-md bg-destructive px-4 py-2 text-sm font-medium text-destructive-foreground hover:bg-destructive/90 focus:outline-none focus:ring-2 focus:ring-destructive focus:ring-offset-2 disabled:opacity-50 disabled:pointer-events-none"
            >
              <Trash2 className="mr-2 h-4 w-4" />
              {isDeleting ? 'Deleting Account...' : 'Delete Account'}
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}

function BrokerageSettingsContent() {
  const {
    connections,
    connectionsLoading,
    connectionsError,
    refetchConnections,
    initiateConnection,
    initiating,
    getConnectionStatus,
    statusLoading,
    deleteConnection,
    deleting,
    completeConnectionSync,
    completingSync,
  } = useBrokerage()

  const [brokerageId, setBrokerageId] = React.useState('')
  const [checkingStatusId, setCheckingStatusId] = React.useState<string | null>(null)
  const [completingSyncId, setCompletingSyncId] = React.useState<string | null>(null)
  const [deletingConnectionId, setDeletingConnectionId] = React.useState<string | null>(null)

  const getStatusBadge = (status: string) => {
    switch (status) {
      case 'connected':
        return (
          <Badge variant="default" className="bg-green-500">
            <CheckCircle2 className="w-3 h-3 mr-1" />
            Connected
          </Badge>
        )
      case 'pending':
        return (
          <Badge variant="outline">
            <Clock className="w-3 h-3 mr-1" />
            Pending
          </Badge>
        )
      case 'disconnected':
        return (
          <Badge variant="destructive">
            <XCircle className="w-3 h-3 mr-1" />
            Disconnected
          </Badge>
        )
      default:
        return <Badge variant="outline">{status}</Badge>
    }
  }

  const handleInitiateConnection = async () => {
    if (!brokerageId.trim()) {
      toast.error('Please enter a brokerage ID')
      return
    }

    try {
      const request: ConnectBrokerageRequest = {
        brokerage_id: brokerageId,
        connection_type: 'read',
      }

      const loadingToast = toast.loading('Initiating connection...')

      const response = await initiateConnection(request)

      if (response?.redirect_url) {
        // Redirect in the same window after receiving the URL
        window.location.href = response.redirect_url

        toast.dismiss(loadingToast)
        toast.success('Opening connection portal...')
      } else {
        toast.dismiss(loadingToast)
        toast.error('Connection initiated but no redirect URL received.')
      }

      setBrokerageId('')
      refetchConnections()
    } catch (error) {
      console.error('Failed to initiate connection:', error)
      toast.error(`Failed to initiate connection: ${error instanceof Error ? error.message : 'Unknown error'}`)
    }
  }

  const handleCheckStatus = async (connectionId: string) => {
    setCheckingStatusId(connectionId)
    try {
      const status = await getConnectionStatus(connectionId)
      toast.success(`Connection status: ${status.status}`)
      refetchConnections()
    } catch (error) {
      console.error('Failed to check status:', error)
      toast.error('Failed to check connection status')
    } finally {
      setCheckingStatusId(null)
    }
  }

  const handleDeleteConnection = async (connectionId: string) => {
    const confirmed = window.confirm(
      'Are you sure you want to disconnect this brokerage account? This will remove the connection but not delete your synced data.'
    )

    if (!confirmed) return

    setDeletingConnectionId(connectionId)
    try {
      await deleteConnection(connectionId)
      toast.success('Brokerage connection removed')
      refetchConnections()
    } catch (error) {
      console.error('Failed to delete connection:', error)
      toast.error('Failed to remove connection. Please try again.')
    } finally {
      setDeletingConnectionId(null)
    }
  }

  const handleCompleteSync = async (connectionId: string) => {
    setCompletingSyncId(connectionId)
    try {
      await completeConnectionSync(connectionId)
      refetchConnections()
      toast.success('Connection synced successfully')
    } catch (error) {
      console.error('Failed to complete sync:', error)
      toast.error('Failed to complete sync')
    } finally {
      setCompletingSyncId(null)
    }
  }

  return (
    <div className="space-y-4">
      {/* Initiate New Connection */}
      <div className="rounded-lg border bg-card p-6">
        <h3 className="text-base font-semibold mb-4">Connect New Brokerage</h3>
        <div className="space-y-4">
          <div className="flex gap-3">
            <div className="flex-1">
              <Input
                placeholder="Brokerage ID (e.g., alderaan, questrade)"
                value={brokerageId}
                onChange={(e) => setBrokerageId(e.target.value)}
              />
            </div>
            <Button
              onClick={handleInitiateConnection}
              disabled={initiating || !brokerageId.trim()}
              size="sm"
            >
              {initiating ? (
                <>
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                  Initiating...
                </>
              ) : (
                <>
                  <Plus className="w-4 h-4 mr-2" />
                  Connect
                </>
              )}
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">
            Enter your brokerage ID and click Connect to start the authentication process.
          </p>
        </div>
      </div>

      {/* Connections List */}
      <div className="rounded-lg border bg-card p-6">
        <div className="flex items-center justify-between mb-2">
          <h3 className="text-base font-semibold">Connected Brokerages</h3>
          <div className="flex items-center gap-3">
            <TooltipProvider>
              <Tooltip delayDuration={200}>
                <TooltipTrigger asChild>
                  <div className="text-xs text-muted-foreground cursor-help max-w-xs">
                    After completing the broker portal, return to this screen and click Sync to finish pulling your data.
                  </div>
                </TooltipTrigger>
                <TooltipContent side="bottom" align="end" className="max-w-xs">
                  After you finish the broker connection flow, come back here and click Sync to complete the data pull.
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
            <Button
              variant="outline"
              size="sm"
              onClick={() => refetchConnections()}
              disabled={connectionsLoading}
              className="h-8"
            >
              <RefreshCw className={`w-4 h-4 mr-2 ${connectionsLoading ? 'animate-spin' : ''}`} />
              Refresh
            </Button>
          </div>
        </div>
        <div className="space-y-4">
          {connectionsLoading ? (
            <div className="space-y-3">
              <div className="h-16 bg-muted rounded animate-pulse" />
              <div className="h-16 bg-muted rounded animate-pulse" />
            </div>
          ) : connectionsError ? (
            <div className="text-destructive py-4 text-sm">
              Error: {connectionsError.message}
            </div>
          ) : connections.length === 0 ? (
            <div className="text-center py-8">
              <LinkIcon className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
              <p className="text-sm text-muted-foreground mb-4">
                No brokerage accounts connected yet.
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {connections.map((connection) => (
                <div
                  key={connection.id}
                  className="flex items-center justify-between p-4 border rounded-lg hover:bg-accent/50 transition-colors"
                >
                  <div className="flex-1">
                    <div className="flex items-center gap-3 mb-2">
                      <h4 className="font-medium">{connection.brokerage_name}</h4>
                      {getStatusBadge(connection.status)}
                    </div>
                    <div className="text-sm text-muted-foreground space-y-1">
                      {connection.last_sync_at && (
                        <div>
                          Last synced: {new Date(connection.last_sync_at).toLocaleString()}
                        </div>
                      )}
                      <div>
                        Connected: {new Date(connection.created_at).toLocaleDateString()}
                      </div>
                    </div>
                  </div>
                  <div className="flex gap-2">
                    {(connection.status === 'connected' || connection.status === 'pending') && (
                      <Button
                        size="sm"
                        onClick={() => handleCompleteSync(connection.id)}
                        disabled={completingSyncId === connection.id || completingSync}
                        variant="outline"
                      >
                        {completingSyncId === connection.id ? (
                          <>
                            <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                            Syncing...
                          </>
                        ) : (
                          <>
                            <ArrowRight className="w-4 h-4 mr-2" />
                            Sync
                          </>
                        )}
                      </Button>
                    )}
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleCheckStatus(connection.id)}
                      disabled={checkingStatusId === connection.id || statusLoading}
                    >
                      {checkingStatusId === connection.id ? (
                        <Loader2 className="w-4 h-4 animate-spin" />
                      ) : (
                        <Activity className="w-4 h-4" />
                      )}
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleDeleteConnection(connection.id)}
                      disabled={deletingConnectionId === connection.id || deleting}
                      className="text-destructive hover:text-destructive"
                    >
                      {deletingConnectionId === connection.id ? (
                        <Loader2 className="w-4 h-4 animate-spin" />
                      ) : (
                        <Trash2 className="w-4 h-4" />
                      )}
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Info Section */}
      <div className="rounded-lg border bg-card p-6">
        <h3 className="text-base font-semibold mb-2">About Brokerage Connections</h3>
        <p className="text-sm text-muted-foreground">
          Connect your brokerage accounts to automatically sync trades, positions, and transactions.
          Your data is securely synced and stored in your personal database.
        </p>
      </div>
    </div>
  )
}

function GeneralSettingsContent() {
  const { theme, setTheme } = useTheme()
  const [mounted, setMounted] = React.useState(false)

  React.useEffect(() => {
    setMounted(true)
  }, [])

  if (!mounted) {
    return (
      <div className="rounded-lg border bg-card p-6">
        <div className="h-20 bg-muted rounded animate-pulse" />
      </div>
    )
  }

  return (
    <div className="space-y-4">
      <div className="rounded-lg border bg-card p-6">
        <h3 className="text-base font-semibold mb-4">Appearance</h3>
        <div className="space-y-4">
          <div className="space-y-2">
            <Label>Theme</Label>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline" className="w-full justify-start">
                  {theme === "light" ? (
                    <Sun className="mr-2 h-4 w-4" />
                  ) : theme === "dark" ? (
                    <Moon className="mr-2 h-4 w-4" />
                  ) : (
                    <Laptop className="mr-2 h-4 w-4" />
                  )}
                  <span>
                    {theme === "light" ? "Light" : theme === "dark" ? "Dark" : "System"}
                  </span>
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start" className="w-56">
                <DropdownMenuRadioGroup
                  value={theme}
                  onValueChange={(value) => setTheme(value as "light" | "dark" | "system")}
                >
                  <DropdownMenuRadioItem value="light" className="flex items-center gap-2">
                    <Sun className="h-4 w-4" />
                    Light
                  </DropdownMenuRadioItem>
                  <DropdownMenuRadioItem value="dark" className="flex items-center gap-2">
                    <Moon className="h-4 w-4" />
                    Dark
                  </DropdownMenuRadioItem>
                  <DropdownMenuRadioItem value="system" className="flex items-center gap-2">
                    <Laptop className="h-4 w-4" />
                    System
                  </DropdownMenuRadioItem>
                </DropdownMenuRadioGroup>
              </DropdownMenuContent>
            </DropdownMenu>
            <p className="text-xs text-muted-foreground">
              Choose how Tradistry looks to you. Select a theme or use your system preference.
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}

export function SettingsDialog({ open, onOpenChange }: SettingsDialogProps) {
  const [activeItem, setActiveItem] = React.useState("General")

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="overflow-hidden p-0 md:max-h-[500px] md:max-w-[700px] lg:max-w-[900px]">
        <DialogTitle className="sr-only">Settings</DialogTitle>
        <DialogDescription className="sr-only">Customize your settings here.</DialogDescription>
        <SidebarProvider className="h-full">
          <div className="flex w-full h-full overflow-hidden">
            <Sidebar className="hidden border-r md:flex w-[200px] shrink-0 relative z-10">
              <SidebarContent>
                <SidebarGroup>
                  <SidebarGroupContent>
                      <SidebarMenu className="space-y-10">
                        {data.nav.map((item) => (
                          <SidebarMenuItem key={item.name}>
                            <SidebarMenuButton
                              isActive={item.name === activeItem}
                              onClick={() => setActiveItem(item.name)}
                              className="transition-colors duration-200"
                            >
                              <a href="#" className="flex items-center gap-3 w-full">
                                <item.icon className="h-4 w-4 shrink-0" />
                                <span className="text-sm font-medium">{item.name}</span>
                              </a>
                            </SidebarMenuButton>
                          </SidebarMenuItem>
                        ))}
                      </SidebarMenu>
                  </SidebarGroupContent>
                </SidebarGroup>
              </SidebarContent>
            </Sidebar>
            <main className="flex h-[480px] flex-1 flex-col overflow-hidden bg-background">
            <header className="flex h-16 shrink-0 items-center gap-2 border-b px-6 transition-[width,height] ease-linear group-has-data-[collapsible=icon]/sidebar-wrapper:h-12">
              <div className="flex items-center gap-2">
                <Breadcrumb>
                  <BreadcrumbList>
                    <BreadcrumbItem className="hidden md:block">
                      <BreadcrumbLink href="#" className="text-sm text-muted-foreground hover:text-foreground">
                        Settings
                      </BreadcrumbLink>
                    </BreadcrumbItem>
                    <BreadcrumbSeparator className="hidden md:block" />
                    <BreadcrumbItem>
                      <BreadcrumbPage className="text-sm font-medium">{activeItem}</BreadcrumbPage>
                    </BreadcrumbItem>
                  </BreadcrumbList>
                </Breadcrumb>
              </div>
            </header>
            <div className="flex flex-1 flex-col gap-6 overflow-y-auto p-6 pt-4">
              <div className="space-y-4">
                <h2 className="text-lg font-semibold tracking-tight">{activeItem}</h2>
                <p className="text-sm text-muted-foreground">
                  Manage your {activeItem.toLowerCase()} preferences and settings.
                </p>
              </div>
              <div className="grid gap-4">
                {activeItem === "Account" ? (
                  <AccountSettingsContent onOpenChange={onOpenChange} />
                ) : activeItem === "General" ? (
                  <GeneralSettingsContent />
                ) : activeItem === "Brokerage" ? (
                  <BrokerageSettingsContent />
                ) : activeItem === "Notifications" ? (
                  <NotificationsPreferences />
                ) : (
                  Array.from({ length: 5 }).map((_, i) => (
                    <div
                      key={i}
                      className="rounded-lg border bg-card p-4 hover:bg-accent/50 transition-colors duration-200"
                    >
                      <div className="h-12 bg-muted rounded animate-pulse" />
                    </div>
                  ))
                )}
              </div>
            </div>
          </main>
          </div>
        </SidebarProvider>
      </DialogContent>
    </Dialog>
  )
}
