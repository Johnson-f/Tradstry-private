# Accounts System Design

## Overview

A trading portfolio/workspace accounts system for Tradstry. Users can create multiple accounts, each representing a separate trading portfolio with its own configuration. The accounts system lives in the sidebar and replaces the current hardcoded team switcher.

## Prerequisites

Install before implementation:
- `zustand` — `bun add zustand`
- shadcn/ui components — `bunx shadcn@latest add dialog select radio-group label`

## Data Model

```typescript
interface Account {
  id: string
  name: string           // e.g., "Main Portfolio", "Swing Trading"
  icon: string           // hugeicons icon name (serializable string), default: "chart-line-data-01"
  currency: Currency     // constrained to supported currencies
  broker: string | null  // linked brokerage name, null if not connected
  riskProfile: "conservative" | "moderate" | "aggressive"
  createdAt: string      // ISO date string
  updatedAt: string      // ISO date string
}

type Currency = "USD" | "EUR" | "GBP" | "JPY" | "CAD" | "AUD" | "CHF"
```

The `icon` field stores a string key that maps to a Hugeicons component via `icon-map.ts`, keeping the data serializable for the Zustand store and future API responses.

### Validation Rules

- **name**: 1–50 characters, required, must be unique across user's accounts
- **currency**: must be one of the `Currency` type values
- **icon**: must exist in `icon-map.ts`, defaults to `"chart-line-data-01"` if not selected
- **riskProfile**: defaults to `"moderate"`

## Architecture

**State management**: Zustand store with `persist` middleware (localStorage) and mock data for initial seed. Clean hook interface so swapping to real API calls later only requires changing the store internals.

### Zustand Store

```typescript
interface AccountStore {
  accounts: Account[]
  activeAccountId: string | null
  setActiveAccount: (id: string) => void
  createAccount: (data: Omit<Account, "id" | "createdAt" | "updatedAt">) => Account
  updateAccount: (id: string, data: Partial<Account>) => void
  deleteAccount: (id: string) => void
}
```

**Delete behavior:**
- Cannot delete the last remaining account (action is a no-op, button is disabled in UI)
- When deleting the currently active account, store auto-switches `activeAccountId` to the first remaining account
- UI shows a confirmation dialog before deletion

### Hooks (thin selectors over the store)

- `useAccounts()` → `Account[]`
- `useActiveAccount()` → `Account | null`
- `useAccountActions()` → `{ create, update, delete, setActive }`

## File Structure

```
src/components/accounts/
├── types.ts                    # Account interface, Currency type, & related types
├── mock-data.ts                # Hardcoded seed accounts for development
├── store.ts                    # Zustand store with persist middleware
├── hooks.ts                    # Convenience hooks
├── icon-map.ts                 # Maps icon name strings → Hugeicons components
├── account-switcher.tsx        # Sidebar dropdown to switch/create accounts
├── account-dialog.tsx          # Shared dialog for creating and editing accounts
└── index.ts                    # Public exports (file already exists, will be populated)
```

## UI Components

### Account Switcher (sidebar header)

Replaces the current `TeamSwitcher`. Reads from Zustand store instead of receiving props.

- Displays active account name, icon, and currency badge
- Dropdown lists all accounts with display-only keyboard shortcut hints (⌘1, ⌘2, etc.)
- "Add account" button at the bottom opens the account dialog in create mode
- Each account in the dropdown has a "..." button for edit/delete

### Account Dialog (create & edit)

Single dialog component used for both creating and editing accounts.

- Fields: name (text input), currency (select dropdown), risk profile (radio group), icon (picker from icon-map)
- Broker field left empty for future brokerage integration
- **Create mode**: on submit calls `createAccount`, auto-switches to the new account
- **Edit mode**: on submit calls `updateAccount` with changed fields

### Delete Confirmation

- Confirmation dialog before deletion ("Are you sure? This will remove all data associated with this account.")
- Delete button is disabled when only one account remains

## Changes to Existing Files

- `src/components/app-sidebar.tsx` — Remove hardcoded `teams` data, import `AccountSwitcher` instead of `TeamSwitcher`
- `src/components/team-switcher.tsx` — Replaced by `account-switcher.tsx`, can be deleted

## Dependencies

- `zustand` — State management with `persist` middleware for localStorage
- shadcn/ui: `dialog`, `select`, `radio-group`, `label` components

## Future Considerations

- Swap mock data for real API calls by changing store internals
- Brokerage linking per account via Snaptrade API
- Account-scoped data filtering (trades, analytics, journal entries)
- Account sharing/collaboration features
- Functional keyboard shortcuts (currently display-only hints)
- Drag-to-reorder accounts (add `order` field to model)
- Account description/notes field
