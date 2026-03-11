"use client"

import { useMemo, useState, type ComponentProps } from "react"
import {
  Home,
  NotebookPen,
  GraduationCap,
  Library,
  PieChart,
  BarChart4,
  BrainCog,
  BookOpen,
  LayoutDashboard,
  TrendingUp,
  Building2,
} from "lucide-react"

import { NavMain } from "@/components/nav-main"
import { NavUser } from "@/components/nav-user"
import { SearchForm } from "@/components/search-form"
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar"
import { useUserProfile } from "@/hooks/use-user-profile"

// Navigation items grouped by category
const mainNavItems = [
  { title: "Home", url: "/app", icon: Home },
  { title: "Dashboard", url: "/app/dashboard", icon: LayoutDashboard },
]

const tradingNavItems = [
  { title: "Journaling", url: "/app/journaling", icon: NotebookPen },
  { title: "Playbook", url: "/app/playbook", icon: Library },
  { title: "Notebook", url: "/app/notebook", icon: BookOpen },
]

const analyticsNavItems = [
  { title: "Analytics", url: "/app/analytics", icon: PieChart },
  { title: "Reporting", url: "/app/reporting", icon: BarChart4 },
]

const resourceNavItems = [
  { title: "Mindset Lab", url: "/app/mindset", icon: BrainCog },
  { title: "Markets", url: "/app/markets", icon: TrendingUp },
  { title: "Brokerage", url: "/app/brokerage", icon: Building2 },
  { title: "Education", url: "/app/education", icon: GraduationCap },
]

export function AppSidebar({ ...props }: ComponentProps<typeof Sidebar>) {
  const { firstName, email } = useUserProfile()
  const [searchTerm, setSearchTerm] = useState("")

  const userData = {
    name: firstName || email?.split('@')[0] || "User",
    avatar: "/placeholder.svg?height=32&width=32",
  }

  const filteredNavItems = useMemo(() => {
    const query = searchTerm.trim().toLowerCase()
    if (!query) {
      return {
        main: mainNavItems,
        trading: tradingNavItems,
        analytics: analyticsNavItems,
        resources: resourceNavItems,
      }
    }

    const filterItems = (items: typeof mainNavItems) =>
      items.filter((item) => item.title.toLowerCase().includes(query))

    return {
      main: filterItems(mainNavItems),
      trading: filterItems(tradingNavItems),
      analytics: filterItems(analyticsNavItems),
      resources: filterItems(resourceNavItems),
    }
  }, [searchTerm])

  return (
    <Sidebar {...props}>
      <SidebarHeader>
        {/* Brand Header */}
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton size="lg" asChild>
              <div className="flex items-center justify-center gap-3 w-full">
                <div className="flex flex-col gap-0.5 leading-none">
                  <span 
                    className="font-black text-2xl bg-clip-text text-transparent"
                    style={{ 
                      letterSpacing: '0.2em',
                      backgroundImage: 'linear-gradient(to right, #FF6B35, #FF6B9D, #C44569, #6C5CE7)'
                    }}
                  >
                    {/* Replace this with your branding*/}
                    YOUR-JOURNAL
                  </span>
                </div>
              </div>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
        <SearchForm value={searchTerm} onChange={setSearchTerm} />
      </SidebarHeader>

      <SidebarContent>
        {filteredNavItems.main.length > 0 && (
          <NavMain items={filteredNavItems.main} label="Overview" />
        )}
        {filteredNavItems.trading.length > 0 && (
          <NavMain items={filteredNavItems.trading} label="Trading" />
        )}
        {filteredNavItems.analytics.length > 0 && (
          <NavMain items={filteredNavItems.analytics} label="Insights" />
        )}
        {filteredNavItems.resources.length > 0 && (
          <NavMain items={filteredNavItems.resources} label="Resources" />
        )}
      </SidebarContent>

      <SidebarFooter>
        <NavUser user={userData} />
      </SidebarFooter>
    </Sidebar>
  )
}