// Phase: 3
// Store provider — Zustand stores are module-level singletons,
// so this is a placeholder for any future context needs.

import type { ReactNode } from "react";

interface StoreProviderProps {
  children: ReactNode;
}

export function StoreProvider({ children }: StoreProviderProps) {
  return <>{children}</>;
}
