"use client";

import {
  Delete02Icon,
  PencilEdit01Icon,
  PlusSignIcon,
  UnfoldMoreIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import * as React from "react";
import { Button } from "@/components/ui/button";
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
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuShortcut,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from "@/components/ui/sidebar";
import { AccountDialog } from "./account-dialog";
import { Skeleton } from "@/components/ui/skeleton";
import {
  useAccountActions,
  useAccounts,
  useAccountsError,
  useAccountsLoading,
  useActiveAccount,
} from "./hooks";
import { ACCOUNT_ICONS } from "./icon-map";
import type { Account } from "./types";

export function AccountSwitcher() {
  const { isMobile } = useSidebar();
  const accounts = useAccounts();
  const activeAccount = useActiveAccount();
  const actions = useAccountActions();
  const isLoading = useAccountsLoading();
  const error = useAccountsError();

  const [dialogOpen, setDialogOpen] = React.useState(false);
  const [editingAccount, setEditingAccount] = React.useState<Account | null>(
    null,
  );
  const [deleteTarget, setDeleteTarget] = React.useState<Account | null>(null);

  function handleEdit(account: Account) {
    setEditingAccount(account);
    setDialogOpen(true);
  }

  function handleCreate() {
    setEditingAccount(null);
    setDialogOpen(true);
  }

  function handleConfirmDelete() {
    if (deleteTarget) {
      actions.delete(deleteTarget.id, accounts);
      setDeleteTarget(null);
    }
  }

  if (isLoading) {
    return (
      <SidebarMenu>
        <SidebarMenuItem>
          <SidebarMenuButton size="lg" disabled>
            <Skeleton className="size-8 rounded-lg" />
            <div className="grid flex-1 gap-1">
              <Skeleton className="h-4 w-24" />
              <Skeleton className="h-3 w-12" />
            </div>
          </SidebarMenuButton>
        </SidebarMenuItem>
      </SidebarMenu>
    );
  }

  if (error) {
    return (
      <SidebarMenu>
        <SidebarMenuItem>
          <SidebarMenuButton size="lg" disabled>
            <div className="grid flex-1 text-left text-sm leading-tight">
              <span className="truncate font-medium">Accounts unavailable</span>
              <span className="truncate text-xs text-destructive">{error}</span>
            </div>
          </SidebarMenuButton>
        </SidebarMenuItem>
      </SidebarMenu>
    );
  }

  const activeIcon = activeAccount ? ACCOUNT_ICONS[activeAccount.icon] : null;

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
                  {activeIcon ? (
                    <HugeiconsIcon icon={activeIcon} strokeWidth={2} />
                  ) : (
                    <HugeiconsIcon icon={PlusSignIcon} strokeWidth={2} />
                  )}
                </div>
                <div className="grid flex-1 text-left text-sm leading-tight">
                  <span className="truncate font-medium">
                    {activeAccount?.name ?? "No account"}
                  </span>
                  <span className="truncate text-xs">
                    {activeAccount?.currency ?? "Create one to get started"}
                  </span>
                </div>
                <HugeiconsIcon
                  icon={UnfoldMoreIcon}
                  strokeWidth={2}
                  className="ml-auto"
                />
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
                const accountIcon = ACCOUNT_ICONS[account.icon];
                return (
                  <DropdownMenuItem
                    key={account.id}
                    onClick={() => actions.setActive(account.id)}
                    className="gap-2 p-2"
                  >
                    <div className="flex size-6 items-center justify-center rounded-md border">
                      {accountIcon && (
                        <HugeiconsIcon
                          icon={accountIcon}
                          strokeWidth={2}
                          className="size-4"
                        />
                      )}
                    </div>
                    <span className="flex-1 truncate">{account.name}</span>
                    <div className="flex items-center gap-1">
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleEdit(account);
                        }}
                        className="rounded-sm p-0.5 opacity-0 transition-opacity hover:bg-accent group-hover:opacity-100 [div:hover>&]:opacity-100"
                      >
                        <HugeiconsIcon
                          icon={PencilEdit01Icon}
                          strokeWidth={2}
                          className="size-3.5"
                        />
                      </button>
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          setDeleteTarget(account);
                        }}
                        disabled={accounts.length <= 1}
                        className="rounded-sm p-0.5 opacity-0 transition-opacity hover:bg-destructive/10 hover:text-destructive disabled:pointer-events-none disabled:opacity-0 group-hover:opacity-100 [div:hover>&]:opacity-100"
                      >
                        <HugeiconsIcon
                          icon={Delete02Icon}
                          strokeWidth={2}
                          className="size-3.5"
                        />
                      </button>
                    </div>
                    <DropdownMenuShortcut>⌘{index + 1}</DropdownMenuShortcut>
                  </DropdownMenuItem>
                );
              })}
              <DropdownMenuSeparator />
              <DropdownMenuItem className="gap-2 p-2" onClick={handleCreate}>
                <div className="flex size-6 items-center justify-center rounded-md border bg-transparent">
                  <HugeiconsIcon
                    icon={PlusSignIcon}
                    strokeWidth={2}
                    className="size-4"
                  />
                </div>
                <div className="font-medium text-muted-foreground">
                  Add account
                </div>
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
              Are you sure you want to delete &quot;{deleteTarget?.name}&quot;?
              This will remove all data associated with this account.
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
  );
}
