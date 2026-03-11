"use client"

import * as React from "react"
import { useState, useEffect, useCallback } from "react"
import { useRouter } from "next/navigation"
import { useTheme } from "next-themes"
import {
  Bell,
  ChevronsUpDown,
  LogOut,
  Settings,
  Moon,
  Sun,
} from "lucide-react"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
// Removed SidebarProvider dependencies since we're using custom implementation
import { createClient } from "@/lib/supabase/client"
import { SettingsDialog } from "./settings-dialog"
import { PriceAlertsDialog } from "@/components/price-alerts-dialog"

 

// Profile types matching backend response
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

interface NavUserProps {
  user: {
    name: string;
    email?: string;
    avatar: string;
  };
  collapsed?: boolean;
}

export function NavUser({
  user,
  collapsed = false,
}: NavUserProps) {
  const router = useRouter()
  const { theme, setTheme } = useTheme()
  
  
  // Simple mobile detection (alternative to useSidebar hook)
  const [isMobile, setIsMobile] = React.useState(false)
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [priceAlertsOpen, setPriceAlertsOpen] = useState(false)
  const [mounted, setMounted] = useState(false)
  const [profile, setProfile] = useState<UserProfileData | null>(null)
  
  // Fetch user profile from Turso database
  useEffect(() => {
    let isMounted = true;

    async function fetchProfile(): Promise<void> {
      try {

        const supabase = createClient();
        const { data: { user: authUser }, error: authError } = await supabase.auth.getUser();
        
        if (authError) {
          throw new Error(`Authentication error: ${authError.message}`);
        }

        if (!authUser) {
          throw new Error('No authenticated user found');
        }

        const response = await fetch(`/api/user/profile/${authUser.id}`, {
          method: 'GET',
          headers: {
            'Content-Type': 'application/json',
          },
        });

        if (!response.ok) {
          const errorData = await response.json().catch(() => ({ error: 'Failed to fetch profile' }));
          throw new Error(errorData.error || `HTTP error! status: ${response.status}`);
        }

        const result: ProfileApiResponse = await response.json();
        
        if (!isMounted) return;

        if (result.success && result.profile) {
          setProfile(result.profile);
        } else {
          // Profile doesn't exist yet or empty response - that's OK, use fallback
          setProfile(null);
        }
      } catch (error) {
        if (!isMounted) return;
        
        console.error('Failed to fetch profile:', error);
        // Don't block UI on profile fetch failure - just log and use fallback
      } finally {
        // Cleanup handled by isMounted check
      }
    }

    fetchProfile();

    return () => {
      isMounted = false;
    };
  }, []);
  
  React.useEffect(() => {
    setMounted(true)
    
    const checkIsMobile = () => {
      setIsMobile(window.innerWidth < 768)
    }
    
    checkIsMobile()
    window.addEventListener('resize', checkIsMobile)
    
    return () => window.removeEventListener('resize', checkIsMobile)
  }, [])

  const handleLogout = async () => {
    const supabase = createClient()
    await supabase.auth.signOut()
    router.push("/auth/login")
  }

  const handleThemeToggle = () => {
    setTheme(theme === "dark" ? "light" : "dark")
  }

  const handlePriceAlerts = useCallback(() => {
    setPriceAlertsOpen(true)
  }, [])

  // Generate initials from user name
  const getInitials = (name: string) => {
    return name
      .split(' ')
      .map(word => word.charAt(0))
      .join('')
      .toUpperCase()
      .slice(0, 2)
  }

  // Use nickname or display_name, fallback to user.name
  const displayName = profile?.nickname || profile?.display_name || user.name
  
  // Get profile picture URL from Supabase storage
  const profilePictureUrl = profile?.profile_picture_uuid 
    ? `${process.env.NEXT_PUBLIC_SUPABASE_URL}/storage/v1/object/public/profile-pictures/${profile.profile_picture_uuid}`
    : null;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          className={`flex items-center gap-3 rounded-lg cursor-pointer text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors w-full px-3 py-2 ${
            collapsed ? "justify-center gap-0 w-10 h-10 p-0" : ""
          }`}
        >
          {profilePictureUrl ? (
            <img 
              src={profilePictureUrl} 
              alt={displayName}
              className="w-8 h-8 rounded-lg object-cover"
              onError={(e) => {
                // Fallback to initials on image load error
                e.currentTarget.style.display = 'none';
                e.currentTarget.nextElementSibling?.classList.remove('hidden');
              }}
            />
          ) : null}
          <div className={`w-8 h-8 rounded-lg bg-gradient-to-br from-purple-600 to-blue-600 flex items-center justify-center text-white font-semibold text-sm ${profilePictureUrl ? 'hidden' : ''}`}>
            {getInitials(displayName)}
          </div>
          {!collapsed && (
            <>
              <div className="grid flex-1 text-left text-sm leading-tight">
                <span className="truncate font-medium text-foreground">{displayName}</span>
                {user.email && (
                <span className="truncate text-xs text-muted-foreground">{user.email}</span>
                )}
              </div>
              <ChevronsUpDown className="ml-auto w-4 h-4" />
            </>
          )}
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        className="w-56 rounded-lg"
        side={isMobile ? "bottom" : "right"}
        align="end"
        sideOffset={4}
      >
        <DropdownMenuLabel className="p-0 font-normal">
          <div className="flex items-center gap-2 px-1 py-1.5 text-left text-sm">
            {profilePictureUrl ? (
              <img 
                src={profilePictureUrl} 
                alt={displayName}
                className="w-8 h-8 rounded-lg object-cover"
                onError={(e) => {
                  e.currentTarget.style.display = 'none';
                  e.currentTarget.nextElementSibling?.classList.remove('hidden');
                }}
              />
            ) : null}
            <div className={`w-8 h-8 rounded-lg bg-gradient-to-br from-purple-600 to-blue-600 flex items-center justify-center text-white font-semibold text-sm ${profilePictureUrl ? 'hidden' : ''}`}>
              {getInitials(displayName)}
            </div>
            <div className="grid flex-1 text-left text-sm leading-tight">
              <span className="truncate font-medium">{displayName}</span>
              {user.email && (
              <span className="truncate text-xs">{user.email}</span>
              )}
            </div>
          </div>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuGroup>
          <DropdownMenuItem onClick={() => setSettingsOpen(true)}>
            <Settings className="mr-2 h-4 w-4" />
            Settings
          </DropdownMenuItem>
          <DropdownMenuItem onClick={handlePriceAlerts}>
            <Bell className="mr-2 h-4 w-4" />
            Price alerts
          </DropdownMenuItem>
          
          <DropdownMenuItem onClick={handleThemeToggle}>
            {mounted && (theme === "dark" ? (
              <Sun className="mr-2 h-4 w-4" />
            ) : (
              <Moon className="mr-2 h-4 w-4" />
            ))}
            {mounted ? (theme === "dark" ? "Light Mode" : "Dark Mode") : "Theme"}
          </DropdownMenuItem>
        </DropdownMenuGroup>
        <DropdownMenuSeparator />
        <DropdownMenuItem onClick={handleLogout}>
          <LogOut className="mr-2 h-4 w-4" />
          Log out
        </DropdownMenuItem>
      </DropdownMenuContent>
      <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
      <PriceAlertsDialog open={priceAlertsOpen} onOpenChange={setPriceAlertsOpen} />
    </DropdownMenu>
  )
}
