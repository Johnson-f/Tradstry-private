# Accounts System Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a multi-account trading portfolio system in the sidebar, replacing the hardcoded team switcher with a Zustand-powered account switcher supporting full CRUD.

**Architecture:** Zustand store with `persist` middleware (localStorage) behind convenience hooks (`useAccounts`, `useActiveAccount`, `useAccountActions`). Mock seed data for development. UI replaces the existing `TeamSwitcher` component with an `AccountSwitcher` that reads from the store.

**Tech Stack:** Next.js 16, React 19, TypeScript, Zustand, shadcn/ui (Dialog, Select, RadioGroup, Label), Hugeicons, Tailwind CSS v4, Biome (linting/formatting)

**Spec:** `docs/superpowers/specs/2026-03-13-accounts-system-design.md`

---

## Chunk 1: Prerequisites & Data Layer

### Task 1: Install dependencies

**Files:** `package.json`, `bun.lock`

- [ ] **Step 1: Install zustand**

```bash
cd /Users/user/Tradstry/frontend && bun add zustand
```

- [ ] **Step 2: Install shadcn/ui components**

```bash
cd /Users/user/Tradstry/frontend && bunx shadcn@latest add dialog select radio-group label
```

Note: The shadcn CLI may prompt for configuration. Accept defaults — the project already has `components.json` configured correctly.

- [ ] **Step 2b: Verify shadcn components were created**

```bash
ls /Users/user/Tradstry/frontend/src/components/ui/dialog.tsx /Users/user/Tradstry/frontend/src/components/ui/select.tsx /Users/user/Tradstry/frontend/src/components/ui/radio-group.tsx /Users/user/Tradstry/frontend/src/components/ui/label.tsx
```

Expected: all 4 files listed.

- [ ] **Step 3: Verify installation**

```bash
cd /Users/user/Tradstry/frontend && bun run biome check src/
```

Expected: no errors related to missing dependencies.

- [ ] **Step 4: Commit**

```bash
git add package.json bun.lock src/components/ui/
git commit -m "chore: add zustand and shadcn dialog/select/radio-group/label"
```

---

### Task 2: Types & constants

**Files:**
- Create: `src/components/accounts/types.ts`

- [ ] **Step 1: Create types file**

```typescript
// src/components/accounts/types.ts

export const CURRENCIES = ["USD", "EUR", "GBP", "JPY", "CAD", "AUD", "CHF"] as const

export type Currency = (typeof CURRENCIES)[number]

export type RiskProfile = "conservative" | "moderate" | "aggressive"

export interface Account {
  id: string
  name: string
  icon: string
  currency: Currency
  broker: string | null
  riskProfile: RiskProfile
  createdAt: string
  updatedAt: string
}
```

- [ ] **Step 2: Verify with biome**

```bash
cd /Users/user/Tradstry/frontend && bun run biome check src/components/accounts/types.ts
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/accounts/types.ts
git commit -m "feat(accounts): add Account type and Currency constants"
```

---

### Task 3: Icon map

**Files:**
- Create: `src/components/accounts/icon-map.ts`

This maps serializable string keys to Hugeicons icon objects. The project uses `@hugeicons/react` with `@hugeicons/core-free-icons`.

- [ ] **Step 1: Create icon map file**

```typescript
// src/components/accounts/icon-map.ts

import {
  ChartLineData01Icon,
  PieChartIcon,
  Analytics01Icon,
  MoneyBag02Icon,
  BitcoinIcon,
  Globe02Icon,
  Target02Icon,
  FlashIcon,
  ShieldCheckIcon,
  TrendUp01Icon,
} from "@hugeicons/core-free-icons"
import type { IconSvgElement } from "@hugeicons/react"

export const ACCOUNT_ICONS: Record<string, IconSvgElement> = {
  "chart-line-data-01": ChartLineData01Icon,
  "pie-chart": PieChartIcon,
  "analytics-01": Analytics01Icon,
  "money-bag-02": MoneyBag02Icon,
  "bitcoin": BitcoinIcon,
  "globe-02": Globe02Icon,
  "target-02": Target02Icon,
  "flash": FlashIcon,
  "shield-check": ShieldCheckIcon,
  "trend-up-01": TrendUp01Icon,
}

export const DEFAULT_ICON = "chart-line-data-01"

export const ICON_OPTIONS = Object.keys(ACCOUNT_ICONS)
```

- [ ] **Step 2: Verify with biome**

```bash
cd /Users/user/Tradstry/frontend && bun run biome check src/components/accounts/icon-map.ts
```

Expected: no errors. **Important:** If some icon imports don't exist in the free icon set (e.g., `ShieldCheckIcon`, `Target02Icon`, `FlashIcon`), the build will fail. Replace any missing imports with available alternatives from `@hugeicons/core-free-icons`. Check availability by running: `grep -r "export.*Icon" node_modules/@hugeicons/core-free-icons/src/ | head -50`

- [ ] **Step 3: Commit**

```bash
git add src/components/accounts/icon-map.ts
git commit -m "feat(accounts): add icon map for account icon picker"
```

---

### Task 4: Mock data

**Files:**
- Create: `src/components/accounts/mock-data.ts`

- [ ] **Step 1: Create mock data file**

```typescript
// src/components/accounts/mock-data.ts

import type { Account } from "./types"

export const MOCK_ACCOUNTS: Account[] = [
  {
    id: "acc_1",
    name: "Main Portfolio",
    icon: "chart-line-data-01",
    currency: "USD",
    broker: null,
    riskProfile: "moderate",
    createdAt: "2026-01-15T10:00:00.000Z",
    updatedAt: "2026-01-15T10:00:00.000Z",
  },
  {
    id: "acc_2",
    name: "Swing Trading",
    icon: "trend-up-01",
    currency: "USD",
    broker: null,
    riskProfile: "aggressive",
    createdAt: "2026-02-01T14:30:00.000Z",
    updatedAt: "2026-02-01T14:30:00.000Z",
  },
  {
    id: "acc_3",
    name: "EUR Investments",
    icon: "globe-02",
    currency: "EUR",
    broker: null,
    riskProfile: "conservative",
    createdAt: "2026-03-01T09:00:00.000Z",
    updatedAt: "2026-03-01T09:00:00.000Z",
  },
]
```

- [ ] **Step 2: Verify with biome**

```bash
cd /Users/user/Tradstry/frontend && bun run biome check src/components/accounts/mock-data.ts
```

- [ ] **Step 3: Commit**

```bash
git add src/components/accounts/mock-data.ts
git commit -m "feat(accounts): add mock account data for development"
```

---

### Task 5: Zustand store

**Files:**
- Create: `src/components/accounts/store.ts`

- [ ] **Step 1: Create the store**

```typescript
// src/components/accounts/store.ts

import { create } from "zustand"
import { persist } from "zustand/middleware"
import type { Account } from "./types"
import { MOCK_ACCOUNTS } from "./mock-data"

interface AccountStore {
  accounts: Account[]
  activeAccountId: string | null
  setActiveAccount: (id: string) => void
  createAccount: (data: Omit<Account, "id" | "createdAt" | "updatedAt">) => Account
  updateAccount: (id: string, data: Partial<Omit<Account, "id" | "createdAt">>) => void
  deleteAccount: (id: string) => void
}

export const useAccountStore = create<AccountStore>()(
  persist(
    (set, get) => ({
      accounts: MOCK_ACCOUNTS,
      activeAccountId: MOCK_ACCOUNTS[0]?.id ?? null,

      setActiveAccount: (id) => {
        const exists = get().accounts.some((a) => a.id === id)
        if (exists) {
          set({ activeAccountId: id })
        }
      },

      createAccount: (data) => {
        const now = new Date().toISOString()
        const account: Account = {
          ...data,
          id: `acc_${crypto.randomUUID()}`,
          createdAt: now,
          updatedAt: now,
        }
        set((state) => ({
          accounts: [...state.accounts, account],
          activeAccountId: account.id,
        }))
        return account
      },

      updateAccount: (id, data) => {
        set((state) => ({
          accounts: state.accounts.map((a) =>
            a.id === id
              ? { ...a, ...data, updatedAt: new Date().toISOString() }
              : a,
          ),
        }))
      },

      deleteAccount: (id) => {
        const { accounts, activeAccountId } = get()
        if (accounts.length <= 1) return

        const remaining = accounts.filter((a) => a.id !== id)
        const newActiveId =
          activeAccountId === id
            ? remaining[0]?.id ?? null
            : activeAccountId

        set({
          accounts: remaining,
          activeAccountId: newActiveId,
        })
      },
    }),
    {
      name: "tradstry-accounts",
    },
  ),
)
```

- [ ] **Step 2: Verify with biome**

```bash
cd /Users/user/Tradstry/frontend && bun run biome check src/components/accounts/store.ts
```

- [ ] **Step 3: Commit**

```bash
git add src/components/accounts/store.ts
git commit -m "feat(accounts): add Zustand store with persist middleware"
```

---

### Task 6: Convenience hooks

**Files:**
- Create: `src/components/accounts/hooks.ts`

- [ ] **Step 1: Create hooks file**

```typescript
// src/components/accounts/hooks.ts

import { useShallow } from "zustand/react/shallow"
import { useAccountStore } from "./store"
import type { Account } from "./types"

export function useAccounts(): Account[] {
  return useAccountStore((state) => state.accounts)
}

export function useActiveAccount(): Account | null {
  return useAccountStore((state) => {
    const { accounts, activeAccountId } = state
    return accounts.find((a) => a.id === activeAccountId) ?? null
  })
}

export function useAccountActions() {
  return useAccountStore(
    useShallow((state) => ({
      setActive: state.setActiveAccount,
      create: state.createAccount,
      update: state.updateAccount,
      delete: state.deleteAccount,
    })),
  )
}
```

- [ ] **Step 2: Verify with biome**

```bash
cd /Users/user/Tradstry/frontend && bun run biome check src/components/accounts/hooks.ts
```

- [ ] **Step 3: Commit**

```bash
git add src/components/accounts/hooks.ts
git commit -m "feat(accounts): add convenience hooks for account store"
```

---

## Chunk 2: UI Components & Integration

### Task 7: Account dialog (create & edit)

**Files:**
- Create: `src/components/accounts/account-dialog.tsx`

This component renders a `Dialog` with form fields for name, currency, risk profile, and icon. It supports both create and edit modes via props.

- [ ] **Step 1: Create the account dialog component**

```tsx
// src/components/accounts/account-dialog.tsx

"use client"

import * as React from "react"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group"
import { HugeiconsIcon } from "@hugeicons/react"
import { ACCOUNT_ICONS, DEFAULT_ICON, ICON_OPTIONS } from "./icon-map"
import { CURRENCIES } from "./types"
import type { Account, Currency, RiskProfile } from "./types"
import { useAccounts, useAccountActions } from "./hooks"

interface AccountDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  account?: Account | null
}

export function AccountDialog({ open, onOpenChange, account }: AccountDialogProps) {
  const isEditing = !!account
  const accounts = useAccounts()
  const actions = useAccountActions()

  const [name, setName] = React.useState("")
  const [currency, setCurrency] = React.useState<Currency>("USD")
  const [riskProfile, setRiskProfile] = React.useState<RiskProfile>("moderate")
  const [icon, setIcon] = React.useState(DEFAULT_ICON)
  const [error, setError] = React.useState("")

  React.useEffect(() => {
    if (account) {
      setName(account.name)
      setCurrency(account.currency)
      setRiskProfile(account.riskProfile)
      setIcon(account.icon)
    } else {
      setName("")
      setCurrency("USD")
      setRiskProfile("moderate")
      setIcon(DEFAULT_ICON)
    }
    setError("")
  }, [account, open])

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault()

    const trimmedName = name.trim()
    if (!trimmedName) {
      setError("Account name is required")
      return
    }
    if (trimmedName.length > 50) {
      setError("Account name must be 50 characters or less")
      return
    }

    const isDuplicate = accounts.some(
      (a) => a.name.toLowerCase() === trimmedName.toLowerCase() && a.id !== account?.id,
    )
    if (isDuplicate) {
      setError("An account with this name already exists")
      return
    }

    if (isEditing && account) {
      actions.update(account.id, {
        name: trimmedName,
        currency,
        riskProfile,
        icon,
      })
    } else {
      actions.create({
        name: trimmedName,
        currency,
        riskProfile,
        icon,
        broker: null,
      })
    }

    onOpenChange(false)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>{isEditing ? "Edit Account" : "Create Account"}</DialogTitle>
            <DialogDescription>
              {isEditing
                ? "Update your trading account settings."
                : "Set up a new trading portfolio."}
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <Label htmlFor="account-name">Name</Label>
              <Input
                id="account-name"
                value={name}
                onChange={(e) => {
                  setName(e.target.value)
                  setError("")
                }}
                placeholder="e.g., Main Portfolio"
                maxLength={50}
              />
              {error && <p className="text-sm text-destructive">{error}</p>}
            </div>

            <div className="grid gap-2">
              <Label>Currency</Label>
              <Select value={currency} onValueChange={(v) => setCurrency(v as Currency)}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {CURRENCIES.map((c) => (
                    <SelectItem key={c} value={c}>
                      {c}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="grid gap-2">
              <Label>Risk Profile</Label>
              <RadioGroup
                value={riskProfile}
                onValueChange={(v) => setRiskProfile(v as RiskProfile)}
                className="flex gap-4"
              >
                <div className="flex items-center gap-2">
                  <RadioGroupItem value="conservative" id="risk-conservative" />
                  <Label htmlFor="risk-conservative" className="font-normal">Conservative</Label>
                </div>
                <div className="flex items-center gap-2">
                  <RadioGroupItem value="moderate" id="risk-moderate" />
                  <Label htmlFor="risk-moderate" className="font-normal">Moderate</Label>
                </div>
                <div className="flex items-center gap-2">
                  <RadioGroupItem value="aggressive" id="risk-aggressive" />
                  <Label htmlFor="risk-aggressive" className="font-normal">Aggressive</Label>
                </div>
              </RadioGroup>
            </div>

            <div className="grid gap-2">
              <Label>Icon</Label>
              <div className="flex flex-wrap gap-2">
                {ICON_OPTIONS.map((key) => (
                  <button
                    key={key}
                    type="button"
                    onClick={() => setIcon(key)}
                    className={`flex size-9 items-center justify-center rounded-md border transition-colors ${
                      icon === key
                        ? "border-primary bg-primary/10 text-primary"
                        : "border-border hover:border-primary/50"
                    }`}
                  >
                    <HugeiconsIcon
                      icon={ACCOUNT_ICONS[key]}
                      strokeWidth={2}
                      className="size-4"
                    />
                  </button>
                ))}
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button type="submit">{isEditing ? "Save Changes" : "Create Account"}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
```

- [ ] **Step 2: Verify with biome**

```bash
cd /Users/user/Tradstry/frontend && bun run biome check src/components/accounts/account-dialog.tsx
```

- [ ] **Step 3: Commit**

```bash
git add src/components/accounts/account-dialog.tsx
git commit -m "feat(accounts): add account dialog for create and edit"
```

---

### Task 8: Account switcher

**Files:**
- Create: `src/components/accounts/account-switcher.tsx`

Replaces `TeamSwitcher`. Reads from Zustand store, renders a dropdown with account list, edit/delete actions, and "Add account" button.

- [ ] **Step 1: Create the account switcher component**

```tsx
// src/components/accounts/account-switcher.tsx

"use client"

import * as React from "react"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuShortcut,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from "@/components/ui/sidebar"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { HugeiconsIcon } from "@hugeicons/react"
import {
  UnfoldMoreIcon,
  PlusSignIcon,
  PencilEdit01Icon,
  Delete02Icon,
} from "@hugeicons/core-free-icons"
import { ACCOUNT_ICONS } from "./icon-map"
import { useAccounts, useActiveAccount, useAccountActions } from "./hooks"
import { AccountDialog } from "./account-dialog"
import type { Account } from "./types"

export function AccountSwitcher() {
  const { isMobile } = useSidebar()
  const accounts = useAccounts()
  const activeAccount = useActiveAccount()
  const actions = useAccountActions()

  const [dialogOpen, setDialogOpen] = React.useState(false)
  const [editingAccount, setEditingAccount] = React.useState<Account | null>(null)
  const [deleteTarget, setDeleteTarget] = React.useState<Account | null>(null)

  function handleEdit(account: Account) {
    setEditingAccount(account)
    setDialogOpen(true)
  }

  function handleCreate() {
    setEditingAccount(null)
    setDialogOpen(true)
  }

  function handleConfirmDelete() {
    if (deleteTarget) {
      actions.delete(deleteTarget.id)
      setDeleteTarget(null)
    }
  }

  if (!activeAccount) return null

  const activeIcon = ACCOUNT_ICONS[activeAccount.icon]

  return (
    <>
      <SidebarMenu>
        <SidebarMenuItem>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <SidebarMenuButton
                size="lg"
                className="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
              >
                <div className="flex aspect-square size-8 items-center justify-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground">
                  {activeIcon && (
                    <HugeiconsIcon icon={activeIcon} strokeWidth={2} />
                  )}
                </div>
                <div className="grid flex-1 text-left text-sm leading-tight">
                  <span className="truncate font-medium">{activeAccount.name}</span>
                  <span className="truncate text-xs">{activeAccount.currency}</span>
                </div>
                <HugeiconsIcon icon={UnfoldMoreIcon} strokeWidth={2} className="ml-auto" />
              </SidebarMenuButton>
            </DropdownMenuTrigger>
            <DropdownMenuContent
              className="w-(--radix-dropdown-menu-trigger-width) min-w-56 rounded-lg"
              align="start"
              side={isMobile ? "bottom" : "right"}
              sideOffset={4}
            >
              <DropdownMenuLabel className="text-xs text-muted-foreground">
                Accounts
              </DropdownMenuLabel>
              {accounts.map((account, index) => {
                const accountIcon = ACCOUNT_ICONS[account.icon]
                return (
                  <DropdownMenuItem
                    key={account.id}
                    onClick={() => actions.setActive(account.id)}
                    className="gap-2 p-2"
                  >
                    <div className="flex size-6 items-center justify-center rounded-md border">
                      {accountIcon && (
                        <HugeiconsIcon icon={accountIcon} strokeWidth={2} className="size-4" />
                      )}
                    </div>
                    <span className="flex-1 truncate">{account.name}</span>
                    <div className="flex items-center gap-1">
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation()
                          handleEdit(account)
                        }}
                        className="rounded-sm p-0.5 opacity-0 transition-opacity hover:bg-accent group-hover:opacity-100 [div:hover>&]:opacity-100"
                      >
                        <HugeiconsIcon icon={PencilEdit01Icon} strokeWidth={2} className="size-3.5" />
                      </button>
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation()
                          setDeleteTarget(account)
                        }}
                        disabled={accounts.length <= 1}
                        className="rounded-sm p-0.5 opacity-0 transition-opacity hover:bg-destructive/10 hover:text-destructive disabled:pointer-events-none disabled:opacity-0 group-hover:opacity-100 [div:hover>&]:opacity-100"
                      >
                        <HugeiconsIcon icon={Delete02Icon} strokeWidth={2} className="size-3.5" />
                      </button>
                    </div>
                    <DropdownMenuShortcut>⌘{index + 1}</DropdownMenuShortcut>
                  </DropdownMenuItem>
                )
              })}
              <DropdownMenuSeparator />
              <DropdownMenuItem className="gap-2 p-2" onClick={handleCreate}>
                <div className="flex size-6 items-center justify-center rounded-md border bg-transparent">
                  <HugeiconsIcon icon={PlusSignIcon} strokeWidth={2} className="size-4" />
                </div>
                <div className="font-medium text-muted-foreground">Add account</div>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </SidebarMenuItem>
      </SidebarMenu>

      <AccountDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        account={editingAccount}
      />

      <Dialog open={!!deleteTarget} onOpenChange={() => setDeleteTarget(null)}>
        <DialogContent className="sm:max-w-[400px]">
          <DialogHeader>
            <DialogTitle>Delete Account</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete "{deleteTarget?.name}"? This will remove all data associated with this account.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteTarget(null)}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={handleConfirmDelete}>
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}
```

- [ ] **Step 2: Verify with biome**

```bash
cd /Users/user/Tradstry/frontend && bun run biome check src/components/accounts/account-switcher.tsx
```

- [ ] **Step 3: Commit**

```bash
git add src/components/accounts/account-switcher.tsx
git commit -m "feat(accounts): add account switcher component for sidebar"
```

---

### Task 9: Barrel exports

**Files:**
- Modify: `src/components/accounts/index.ts` (already exists, currently empty)

- [ ] **Step 1: Populate index.ts with public exports**

```typescript
// src/components/accounts/index.ts

export { AccountSwitcher } from "./account-switcher"
export { AccountDialog } from "./account-dialog"
export { useAccounts, useActiveAccount, useAccountActions } from "./hooks"
export type { Account, Currency, RiskProfile } from "./types"
```

- [ ] **Step 2: Verify with biome**

```bash
cd /Users/user/Tradstry/frontend && bun run biome check src/components/accounts/index.ts
```

- [ ] **Step 3: Commit**

```bash
git add src/components/accounts/index.ts
git commit -m "feat(accounts): add barrel exports"
```

---

### Task 10: Integrate into sidebar & remove old team switcher

**Files:**
- Modify: `src/components/app-sidebar.tsx`
- Delete: `src/components/team-switcher.tsx`

- [ ] **Step 1: Update app-sidebar.tsx**

Replace the `TeamSwitcher` import and usage with `AccountSwitcher`. Remove the hardcoded `teams` data and the unused icon imports (`LayoutBottomIcon`, `AudioWave01Icon`, `CommandIcon`).

The updated file should look like:

```tsx
// src/components/app-sidebar.tsx

"use client"

import * as React from "react"

import { NavMain } from "@/components/nav-main"
import { NavProjects } from "@/components/nav-projects"
import { NavUser } from "@/components/nav-user"
import { AccountSwitcher } from "@/components/accounts"
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarRail,
} from "@/components/ui/sidebar"
import { HugeiconsIcon } from "@hugeicons/react"
import { ComputerTerminalIcon, RoboticIcon, BookOpen02Icon, Settings05Icon, CropIcon, PieChartIcon, MapsIcon } from "@hugeicons/core-free-icons"

// This is sample data.
const data = {
  user: {
    name: "shadcn",
    email: "m@example.com",
    avatar: "/avatars/shadcn.jpg",
  },
  navMain: [
    {
      title: "Playground",
      url: "#",
      icon: (
        <HugeiconsIcon icon={ComputerTerminalIcon} strokeWidth={2} />
      ),
      isActive: true,
      items: [
        { title: "History", url: "#" },
        { title: "Starred", url: "#" },
        { title: "Settings", url: "#" },
      ],
    },
    {
      title: "Models",
      url: "#",
      icon: (
        <HugeiconsIcon icon={RoboticIcon} strokeWidth={2} />
      ),
      items: [
        { title: "Genesis", url: "#" },
        { title: "Explorer", url: "#" },
        { title: "Quantum", url: "#" },
      ],
    },
    {
      title: "Documentation",
      url: "#",
      icon: (
        <HugeiconsIcon icon={BookOpen02Icon} strokeWidth={2} />
      ),
      items: [
        { title: "Introduction", url: "#" },
        { title: "Get Started", url: "#" },
        { title: "Tutorials", url: "#" },
        { title: "Changelog", url: "#" },
      ],
    },
    {
      title: "Settings",
      url: "#",
      icon: (
        <HugeiconsIcon icon={Settings05Icon} strokeWidth={2} />
      ),
      items: [
        { title: "General", url: "#" },
        { title: "Team", url: "#" },
        { title: "Billing", url: "#" },
        { title: "Limits", url: "#" },
      ],
    },
  ],
  projects: [
    {
      name: "Design Engineering",
      url: "#",
      icon: (
        <HugeiconsIcon icon={CropIcon} strokeWidth={2} />
      ),
    },
    {
      name: "Sales & Marketing",
      url: "#",
      icon: (
        <HugeiconsIcon icon={PieChartIcon} strokeWidth={2} />
      ),
    },
    {
      name: "Travel",
      url: "#",
      icon: (
        <HugeiconsIcon icon={MapsIcon} strokeWidth={2} />
      ),
    },
  ],
}

export function AppSidebar({ ...props }: React.ComponentProps<typeof Sidebar>) {
  return (
    <Sidebar collapsible="icon" {...props}>
      <SidebarHeader>
        <AccountSwitcher />
      </SidebarHeader>
      <SidebarContent>
        <NavMain items={data.navMain} />
        <NavProjects projects={data.projects} />
      </SidebarContent>
      <SidebarFooter>
        <NavUser user={data.user} />
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  )
}
```

- [ ] **Step 2: Delete team-switcher.tsx**

```bash
rm /Users/user/Tradstry/frontend/src/components/team-switcher.tsx
```

- [ ] **Step 3: Verify linting and types**

```bash
cd /Users/user/Tradstry/frontend && bun run biome check src/
```

Expected: no errors. If any imports reference `team-switcher`, they need to be cleaned up.

- [ ] **Step 4: Verify the project builds**

```bash
cd /Users/user/Tradstry/frontend && bun run build
```

Expected: build succeeds with no TypeScript or import errors.

- [ ] **Step 5: Commit**

```bash
git add src/components/app-sidebar.tsx src/components/accounts/
git rm src/components/team-switcher.tsx
git commit -m "feat(accounts): integrate account switcher into sidebar, remove team switcher"
```

---

### Task 11: Visual verification

- [ ] **Step 1: Start the dev server**

```bash
cd /Users/user/Tradstry/frontend && bun run dev
```

- [ ] **Step 2: Verify in browser**

Navigate to `http://localhost:3000/dashboard` and verify:
1. The sidebar header shows "Main Portfolio" with the chart icon and "USD"
2. Clicking opens a dropdown listing all 3 mock accounts
3. Switching accounts updates the sidebar header
4. "Add account" opens the create dialog with all form fields
5. Edit (pencil) icon on hover opens the dialog pre-filled
6. Delete (trash) icon shows confirmation, deleting works and auto-switches
7. Cannot delete the last account (button disabled)
8. Refresh the page — accounts persist via localStorage

- [ ] **Step 3: Final commit if any fixes were needed**

```bash
git add -A
git commit -m "fix(accounts): polish from visual verification"
```
