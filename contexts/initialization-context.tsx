'use client'

import { createContext, useContext } from 'react';

export interface InitializationStatus {
  isInitialized: boolean;
  isInitializing: boolean;
  error: string | null;
}

export const InitializationContext = createContext<InitializationStatus>({
  isInitialized: false,
  isInitializing: true,
  error: null,
});

export function useInitializationStatus() {
  return useContext(InitializationContext);
}
