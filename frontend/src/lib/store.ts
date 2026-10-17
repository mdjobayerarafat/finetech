"use client";

import { create } from "zustand";
import type { MeOut } from "./api";

type AuthState = {
  me: MeOut | null;
  loading: boolean;
  setMe: (me: MeOut | null) => void;
  setLoading: (loading: boolean) => void;
  reset: () => void;
};

export const useAuth = create<AuthState>((set) => ({
  me: null,
  loading: true,
  setMe: (me) => set({ me, loading: false }),
  setLoading: (loading) => set({ loading }),
  reset: () => set({ me: null, loading: false }),
}));

// ---------------------------------------------------------------------------
// Cross-page "accounts changed" bus.
//
// Pages that mutate money (transfers, subscriptions, wealth, savings goals)
// call `accountsBusApi.bump()` after a successful action. Pages that render
// account state (dashboard, accounts, transfers) subscribe to the `tick`
// via `useAccountsBusTick()` and re-fetch. This is intentionally tiny —
// no event payloads, just a monotonically-increasing counter so useEffect
// deps fire and React schedules the re-fetch.
// ---------------------------------------------------------------------------
type AccountsBusState = {
  tick: number;
  lastReason: string | null;
  bump: (reason?: string) => void;
};

const _accountsBusStore = create<AccountsBusState>((set) => ({
  tick: 0,
  lastReason: null,
  bump: (reason) =>
    set((s) => ({ tick: s.tick + 1, lastReason: reason ?? null })),
}));

/** React hook — subscribe to the bus tick. */
export const useAccountsBusTick = () => _accountsBusStore((s) => s.tick);

/** Plain object — call this from event handlers (not in render). */
export const accountsBusApi = {
  bump: (reason?: string) => _accountsBusStore.getState().bump(reason),
};
