'use client';

import React from 'react';
import { toast } from 'sonner';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { useDeleteStock } from '@/lib/hooks/use-stocks';
import { useDeleteOption } from '@/lib/hooks/use-options';
import type { Stock } from '@/lib/types/stocks';
import type { OptionTrade } from '@/lib/types/options';

interface DeleteTradeModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  trade: Stock | OptionTrade | null;
  tradeType: 'stock' | 'option';
}

export default function DeleteTradeModal({ open, onOpenChange, trade, tradeType }: DeleteTradeModalProps) {
  const deleteStock = useDeleteStock();
  const deleteOption = useDeleteOption();

  const handleDelete = async () => {
    if (!trade) return;

    try {
      if (tradeType === 'stock') {
        await deleteStock.mutateAsync((trade as Stock).id);
        toast.success('Stock trade deleted successfully');
      } else {
        await deleteOption.mutateAsync((trade as OptionTrade).id);
        toast.success('Option trade deleted successfully');
      }
      onOpenChange(false);
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Failed to delete trade';
      toast.error(message);
      console.error('Error deleting trade:', error);
    }
  };

  if (!trade) return null;

  const tradeSymbol = trade.symbol;
  const isLoading = tradeType === 'stock' ? deleteStock.isPending : deleteOption.isPending;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Delete Trade</DialogTitle>
          <DialogDescription>
            Are you sure you want to delete this {tradeType} trade for <strong>{tradeSymbol}</strong>? This action cannot be undone.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={isLoading}>
            Cancel
          </Button>
          <Button type="button" variant="destructive" onClick={handleDelete} disabled={isLoading}>
            {isLoading ? 'Deleting...' : 'Delete'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

