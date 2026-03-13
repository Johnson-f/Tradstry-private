import { create } from "zustand";
import { persist } from "zustand/middleware";

interface ActiveAccountStore {
  activeAccountId: string | null;
  setActiveAccountId: (id: string) => void;
  clearIfDeleted: (deletedId: string, fallbackId: string | null) => void;
}

export const useActiveAccountStore = create<ActiveAccountStore>()(
  persist(
    (set, get) => ({
      activeAccountId: null,

      setActiveAccountId: (id) => {
        set({ activeAccountId: id });
      },

      clearIfDeleted: (deletedId, fallbackId) => {
        if (get().activeAccountId === deletedId) {
          set({ activeAccountId: fallbackId });
        }
      },
    }),
    {
      name: "tradstry-active-account",
    },
  ),
);
