"use client";

import { useActiveAccount } from "@/components/accounts";
import { JournalTable } from "./journal-table";

export function Journal() {
  const activeAccount = useActiveAccount();

  return <JournalTable key={activeAccount?.id ?? "no-account"} />;
}
