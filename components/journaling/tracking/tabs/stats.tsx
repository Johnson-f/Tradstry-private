'use client';

import React from 'react';
import { Card, CardContent } from '@/components/ui/card';
import { useIndividualTradeAnalytics } from '@/lib/hooks/use-analytics';
import { useAnalyticsCore } from '@/lib/hooks/use-analytics';
import { usePlaybooks } from '@/lib/hooks/use-playbooks';
import { useUpdateStock } from '@/lib/hooks/use-stocks';
import { useUpdateOption } from '@/lib/hooks/use-options';
import { useQueryClient } from '@tanstack/react-query';
import { formatCurrency } from '@/lib/utils';
import { Skeleton } from '@/components/ui/skeleton';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Pencil, Check, X } from 'lucide-react';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Star } from 'lucide-react';
import playbookService from '@/lib/services/playbook-service';
import { toast } from 'sonner';
import type { Stock } from '@/lib/types/stocks';
import type { OptionTrade } from '@/lib/types/options';
import type { Playbook } from '@/lib/types/playbook';
import TradeTagsManager from '@/components/journaling/trade-tags-manager';

interface StatsProps {
  tradeId: number;
  tradeType: 'stock' | 'option';
  trade: Stock | OptionTrade | null;
}

function formatRMultiple(value: number | null | undefined): string {
  if (value === null || value === undefined) return 'N/A';
  return `${value.toFixed(2)}R`;
}

function calculateNetPnL(trade: Stock | OptionTrade | null): number {
  if (!trade) return 0;

  // Check if it's a Stock trade
  if ('tradeType' in trade) {
    const stock = trade as Stock;
    const entryPrice = parseFloat(stock.entryPrice);
    const exitPriceNum = parseFloat(stock.exitPrice);
    const shares = parseFloat(stock.numberShares);
    const commissions = parseFloat(stock.commissions);
    
    const tradeValue = stock.tradeType === 'BUY' 
      ? (exitPriceNum - entryPrice) * shares
      : (entryPrice - exitPriceNum) * shares;
    
    return tradeValue - commissions;
  }
  
  // Check if it's an Option trade
  if ('strategyType' in trade) {
    const option = trade as OptionTrade;
    const entryPriceNum = parseFloat(option.entryPrice);
    const exitPriceNum = parseFloat(option.exitPrice);
    const contracts = option.totalQuantity ? parseFloat(option.totalQuantity) : 1;
    const commissions = parseFloat(option.commissions || '0');
    
    // Each contract represents 100 shares
    const tradeValue = (exitPriceNum - entryPriceNum) * contracts * 100;
    return tradeValue - commissions;
  }

  return 0;
}

export default function Stats({ tradeId, tradeType, trade }: StatsProps) {
  const { data: tradeAnalytics, isLoading: isLoadingAnalytics } = useIndividualTradeAnalytics(
    tradeId,
    tradeType,
    tradeId > 0
  );

  const { data: coreAnalytics, isLoading: isLoadingCore } = useAnalyticsCore();
  const playbooksQuery = usePlaybooks();
  const playbooks = playbooksQuery.data || [];
  const isLoadingPlaybooks = playbooksQuery.isLoading;
  const [selectedPlaybookId, setSelectedPlaybookId] = React.useState<string | null>(null);
  const [isLoadingTradePlaybooks, setIsLoadingTradePlaybooks] = React.useState(true);
  
  // Edit state
  const [isEditing, setIsEditing] = React.useState<Record<string, boolean>>({});
  const [editValues, setEditValues] = React.useState<Record<string, string>>({});
  
  const updateStockMutation = useUpdateStock();
  const updateOptionMutation = useUpdateOption();
  const queryClient = useQueryClient();

  // Handle edit start
  const handleEditStart = (field: string, currentValue: string | number | null) => {
    setIsEditing({ ...isEditing, [field]: true });
    setEditValues({ ...editValues, [field]: currentValue?.toString() || '' });
  };

  // Handle edit cancel
  const handleEditCancel = (field: string) => {
    setIsEditing({ ...isEditing, [field]: false });
    setEditValues({ ...editValues, [field]: '' });
  };

  // Handle save
  const handleSave = async (field: string, value: string) => {
    if (!trade) return;

    // Handle empty values - set to null for optional fields
    if (!value || value.trim() === '') {
      if (field === 'profitTarget' || field === 'initialTarget') {
        // These can be null
        const updates: Record<string, unknown> = {};
        updates[field] = null;
        
        if (tradeType === 'stock') {
          await updateStockMutation.mutateAsync({ id: tradeId, updates });
        } else {
          await updateOptionMutation.mutateAsync({ id: tradeId, updates });
        }
        
        queryClient.invalidateQueries({ queryKey: [tradeType === 'stock' ? 'stocks' : 'options', tradeId] });
        queryClient.invalidateQueries({ queryKey: [tradeType === 'stock' ? 'stocks' : 'options'] });
        
        setIsEditing({ ...isEditing, [field]: false });
        toast.success('Updated successfully');
        return;
      }
      toast.error('Please enter a valid number');
      return;
    }

    const numValue = parseFloat(value);
    if (isNaN(numValue) && field !== 'tradeRatings') {
      toast.error('Please enter a valid number');
      return;
    }

    try {
      const updates: Record<string, unknown> = {};
      
      if (field === 'tradeRatings') {
        updates.tradeRatings = value ? parseInt(value, 10) : null;
      } else if (field === 'profitTarget') {
        updates.profitTarget = numValue;
      } else if (field === 'stopLoss') {
        updates.stopLoss = numValue;
      } else if (field === 'initialTarget') {
        updates.initialTarget = numValue;
      }

      if (tradeType === 'stock') {
        await updateStockMutation.mutateAsync({ id: tradeId, updates });
      } else {
        await updateOptionMutation.mutateAsync({ id: tradeId, updates });
      }

      // Invalidate relevant queries
      queryClient.invalidateQueries({ queryKey: [tradeType === 'stock' ? 'stocks' : 'options', tradeId] });
      queryClient.invalidateQueries({ queryKey: [tradeType === 'stock' ? 'stocks' : 'options'] });

      setIsEditing({ ...isEditing, [field]: false });
      toast.success('Updated successfully');
    } catch (error) {
      toast.error('Failed to update');
      console.error('Error updating:', error);
    }
  };

  // Handle star rating click
  const handleStarClick = async (rating: number) => {
    if (!trade) return;

    try {
      const updates: Record<string, unknown> = {
        tradeRatings: rating,
      };

      if (tradeType === 'stock') {
        await updateStockMutation.mutateAsync({ id: tradeId, updates });
      } else {
        await updateOptionMutation.mutateAsync({ id: tradeId, updates });
      }

      queryClient.invalidateQueries({ queryKey: [tradeType === 'stock' ? 'stocks' : 'options', tradeId] });
      queryClient.invalidateQueries({ queryKey: [tradeType === 'stock' ? 'stocks' : 'options'] });

      toast.success('Rating updated');
    } catch (error) {
      toast.error('Failed to update rating');
      console.error('Error updating rating:', error);
    }
  };

  // Fetch current playbooks for this trade
  React.useEffect(() => {
    if (tradeId > 0) {
      setIsLoadingTradePlaybooks(true);
      playbookService
        .getTradePlaybooks(tradeId, tradeType)
        .then((res) => {
          if (res.success && res.data && res.data.length > 0) {
            // If trade has playbooks, select the first one
            setSelectedPlaybookId(res.data[0].id);
          } else {
            setSelectedPlaybookId(null);
          }
        })
        .catch(() => {
          setSelectedPlaybookId(null);
        })
        .finally(() => {
          setIsLoadingTradePlaybooks(false);
        });
    }
  }, [tradeId, tradeType]);

  const handlePlaybookChange = async (playbookId: string) => {
    if (playbookId === selectedPlaybookId) return;

    try {
      // If there was a previous playbook, untag it first
      if (selectedPlaybookId) {
        await playbookService.untagTrade({
          trade_id: tradeId,
          setup_id: selectedPlaybookId,
          trade_type: tradeType,
        });
      }

      // Tag the new playbook
      if (playbookId && playbookId !== 'none') {
        const result = await playbookService.tagTrade({
          trade_id: tradeId,
          setup_id: playbookId,
          trade_type: tradeType,
        });

        if (result.success) {
          setSelectedPlaybookId(playbookId);
          toast.success('Playbook tagged successfully');
        } else {
          toast.error(result.message || 'Failed to tag playbook');
        }
      } else {
        // User selected "None"
        setSelectedPlaybookId(null);
        if (selectedPlaybookId) {
          toast.success('Playbook untagged successfully');
        }
      }
    } catch (error) {
      toast.error('Failed to update playbook');
      console.error('Error updating playbook:', error);
    }
  };

  const isLoading = isLoadingAnalytics || isLoadingCore;
  const netPnL = calculateNetPnL(trade);
  const averageWin = coreAnalytics?.average_win ?? 0;
  const averageLoss = coreAnalytics?.average_loss ?? 0;
  const plannedRR = tradeAnalytics?.planned_risk_to_reward ?? null;
  const realizedRR = tradeAnalytics?.realized_risk_to_reward ?? null;

  // Get side information
  const side = trade
    ? tradeType === 'stock'
      ? (trade as Stock).tradeType
      : (trade as OptionTrade).optionType
    : null;

  // Get shares/contracts
  const sharesOrContracts = trade
    ? tradeType === 'stock'
      ? parseFloat((trade as Stock).numberShares)
      // @ts-ignore
      : (trade as OptionTrade).numberOfContracts
    : null;

  // Get trade ratings
  const tradeRatings = trade?.tradeRatings ?? null;

  // Get profit target, stop loss, and initial target
  const profitTarget = trade
    ? tradeType === 'stock'
      ? (trade as Stock).profitTarget
      : (trade as OptionTrade).profitTarget
    : null;

  const stopLoss = trade
    ? tradeType === 'stock'
      ? (trade as Stock).stopLoss
      : null // Options don't have stopLoss in the type definition
    : null;

  const initialTarget = trade
    ? tradeType === 'stock'
      ? (trade as Stock).initialTarget
      : (trade as OptionTrade).initialTarget
    : null;

  if (isLoading) {
    return (
      <Card className="shadow-none min-h-[600px]">
        <CardContent className="p-6 space-y-6">
          <div className="space-y-2">
            <Skeleton className="h-4 w-24" />
            <Skeleton className="h-10 w-32" />
          </div>
          <div className="space-y-2">
            <Skeleton className="h-4 w-32" />
            <Skeleton className="h-16 w-full" />
          </div>
          <div className="space-y-2">
            <Skeleton className="h-4 w-28" />
            <Skeleton className="h-6 w-16" />
          </div>
          <div className="space-y-2">
            <Skeleton className="h-4 w-32" />
            <Skeleton className="h-6 w-16" />
          </div>
        </CardContent>
      </Card>
    );
  }

  const isNegative = netPnL < 0;

  return (
    <Card className="bg-white shadow-none min-h-[600px]">
      <CardContent className="p-6 space-y-6">
        {/* Net P&L */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className={`w-1 h-6 rounded ${isNegative ? 'bg-red-500' : 'bg-green-500'}`} />
            <span className="text-sm font-medium text-muted-foreground">Net P&L</span>
          </div>
          <div className={`text-3xl font-bold ${isNegative ? 'text-red-600' : 'text-green-600'}`}>
            {formatCurrency(netPnL)}
          </div>
        </div>

        {/* Side */}
        {side && (
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium text-muted-foreground">Side</span>
            <div className="text-lg font-semibold">{side}</div>
          </div>
        )}

        {/* Shares/Contracts */}
        {sharesOrContracts !== null && (
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium text-muted-foreground">
              {tradeType === 'stock' ? 'Shares' : 'Contracts'}
            </span>
            <div className="text-lg font-semibold">{sharesOrContracts}</div>
          </div>
        )}

        {/* Playbook */}
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium text-muted-foreground">Playbook</span>
          {isLoadingPlaybooks || isLoadingTradePlaybooks ? (
            <Skeleton className="h-9 w-32" />
          ) : (
            <Select
              value={selectedPlaybookId || 'none'}
              onValueChange={handlePlaybookChange}
            >
              <SelectTrigger className="w-[200px]">
                <SelectValue placeholder="Select Playbook" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="none">Select Playbook</SelectItem>
                {playbooks.map((playbook: Playbook) => (
                  <SelectItem key={playbook.id} value={playbook.id}>
                    {playbook.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        </div>

        {/* Trade Ratings */}
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium text-muted-foreground">Trade Rating</span>
          <div className="flex items-center gap-1">
            {[1, 2, 3, 4, 5].map((star) => (
              <Star
                key={star}
                onClick={() => handleStarClick(star)}
                className={`h-4 w-4 cursor-pointer transition-colors ${
                  tradeRatings && star <= tradeRatings
                    ? 'fill-yellow-400 text-yellow-400'
                    : 'fill-none text-muted-foreground'
                } hover:text-yellow-400`}
              />
            ))}
          </div>
        </div>

        {/* Profit Target */}
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium text-muted-foreground">Profit Target</span>
          {isEditing.profitTarget ? (
            <div className="flex items-center gap-2">
              <Input
                type="number"
                step="0.01"
                value={editValues.profitTarget || ''}
                onChange={(e) => setEditValues({ ...editValues, profitTarget: e.target.value })}
                className="w-32 h-8"
                placeholder="0.00"
                autoFocus
              />
              <Button
                size="sm"
                variant="ghost"
                onClick={() => handleSave('profitTarget', editValues.profitTarget || '')}
                className="h-7 w-7 p-0"
              >
                <Check className="h-4 w-4" />
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => handleEditCancel('profitTarget')}
                className="h-7 w-7 p-0"
              >
                <X className="h-4 w-4" />
              </Button>
            </div>
          ) : (
            <div className="flex items-center gap-2">
              <div className="text-lg font-semibold">
                {profitTarget ? formatCurrency(parseFloat(profitTarget)) : 'N/A'}
              </div>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => handleEditStart('profitTarget', profitTarget || '')}
                className="h-7 w-7 p-0"
              >
                <Pencil className="h-3 w-3" />
              </Button>
            </div>
          )}
        </div>

        {/* Stop Loss */}
        {(stopLoss || tradeType === 'stock') && (
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium text-muted-foreground">Stop Loss</span>
            {isEditing.stopLoss ? (
              <div className="flex items-center gap-2">
                <Input
                  type="number"
                  step="0.01"
                  value={editValues.stopLoss || ''}
                  onChange={(e) => setEditValues({ ...editValues, stopLoss: e.target.value })}
                  className="w-32 h-8"
                  placeholder="0.00"
                  autoFocus
                />
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => handleSave('stopLoss', editValues.stopLoss || '')}
                  className="h-7 w-7 p-0"
                >
                  <Check className="h-4 w-4" />
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => handleEditCancel('stopLoss')}
                  className="h-7 w-7 p-0"
                >
                  <X className="h-4 w-4" />
                </Button>
              </div>
            ) : (
              <div className="flex items-center gap-2">
                <div className="text-lg font-semibold">
                  {stopLoss ? formatCurrency(parseFloat(stopLoss)) : 'N/A'}
                </div>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => handleEditStart('stopLoss', stopLoss || '')}
                  className="h-7 w-7 p-0"
                >
                  <Pencil className="h-3 w-3" />
                </Button>
              </div>
            )}
          </div>
        )}

        {/* Initial Target */}
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium text-muted-foreground">Initial Target</span>
          {isEditing.initialTarget ? (
            <div className="flex items-center gap-2">
              <Input
                type="number"
                step="0.01"
                value={editValues.initialTarget || ''}
                onChange={(e) => setEditValues({ ...editValues, initialTarget: e.target.value })}
                className="w-32 h-8"
                placeholder="0.00"
                autoFocus
              />
              <Button
                size="sm"
                variant="ghost"
                onClick={() => handleSave('initialTarget', editValues.initialTarget || '')}
                className="h-7 w-7 p-0"
              >
                <Check className="h-4 w-4" />
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => handleEditCancel('initialTarget')}
                className="h-7 w-7 p-0"
              >
                <X className="h-4 w-4" />
              </Button>
            </div>
          ) : (
            <div className="flex items-center gap-2">
              <div className="text-lg font-semibold">
                {initialTarget ? formatCurrency(parseFloat(initialTarget)) : 'N/A'}
              </div>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => handleEditStart('initialTarget', initialTarget || '')}
                className="h-7 w-7 p-0"
              >
                <Pencil className="h-3 w-3" />
              </Button>
            </div>
          )}
        </div>

        {/* Avg loss/win barchart */}
        <div className="space-y-2">
          <span className="text-sm font-medium text-muted-foreground">Avg loss/win</span>
          <div className="flex items-center gap-2">
            {/* Loss bar (red) - left side */}
            <div 
              className="h-9 rounded bg-red-50 border border-red-200 min-w-[100px] flex items-center justify-center px-3"
              style={{ 
                flex: averageLoss !== 0 && averageWin !== 0 
                  ? `${Math.abs(averageLoss)}` 
                  : averageLoss !== 0 ? '1' : '0',
              }}
            >
              <span className="text-sm font-semibold text-red-700">
                {formatCurrency(Math.abs(averageLoss))}
              </span>
            </div>
            
            {/* Win bar (green) - right side */}
            <div 
              className="h-9 rounded bg-green-50 border border-green-200 min-w-[100px] flex items-center justify-center px-3"
              style={{ 
                flex: averageLoss !== 0 && averageWin !== 0 
                  ? `${Math.abs(averageWin)}` 
                  : averageWin !== 0 ? '1' : '0',
              }}
            >
              <span className="text-sm font-semibold text-green-700">
                {formatCurrency(averageWin)}
              </span>
            </div>
          </div>
        </div>

        {/* Planned R-Multiple */}
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium text-muted-foreground">Planned R-Multiple</span>
          <div className="text-lg font-semibold">
            {formatRMultiple(plannedRR)}
          </div>
        </div>

        {/* Realized R-Multiple */}
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium text-muted-foreground">Realized R-Multiple</span>
          <div className={`text-lg font-semibold ${realizedRR !== null && realizedRR < 0 ? 'text-red-600' : ''}`}>
            {formatRMultiple(realizedRR)}
          </div>
        </div>

        {/* Trade Tags Manager */}
        <div className="pt-6 border-t">
          <TradeTagsManager tradeId={tradeId} tradeType={tradeType} />
        </div>
      </CardContent>
    </Card>
  );
}

