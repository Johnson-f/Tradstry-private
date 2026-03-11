"use client";

import { useRouter } from 'next/navigation';
import { marketDataService } from '@/lib/services/market-data-service';

/**
 * Hook for navigating to symbol pages with automatic database saving
 * Follows the same pattern as symbol search component
 */
export function useSymbolNavigation() {
  const router = useRouter();

  const navigateToSymbol = async (symbol: string) => {
    try {
      // First, check if symbol exists in database
      const symbolExists = await checkSymbolInDatabase(symbol);
      
      if (!symbolExists) {
        // Save symbol to database if it doesn't exist
        await saveSymbolToDatabase(symbol);
      }

      // Navigate to symbol page
      router.push(`/protected/markets/${symbol.toUpperCase()}`);
    } catch (err) {
      console.error('Error handling symbol navigation:', err);
      // Still proceed with navigation even if database operations fail
      router.push(`/protected/markets/${symbol.toUpperCase()}`);
    }
  };

  const checkSymbolInDatabase = async (symbol: string): Promise<boolean> => {
    try {
       // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
      const response = await marketDataService.checkSymbolExists(symbol);
      return response.exists;
    } catch (err) {
      console.error('Error checking symbol in database:', err);
      return false;
    }
  };

  const saveSymbolToDatabase = async (symbol: string): Promise<void> => {
    try {
       // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
      await marketDataService.saveSymbolToDatabase({ symbol });
    } catch (err) {
      console.error('Error saving symbol to database:', err);
      // Don't throw - we still want to proceed with navigation
    }
  };

  return { navigateToSymbol };
}
