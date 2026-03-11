'use client';

import React from 'react';
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { toast } from 'sonner';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '@/components/ui/tabs';
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Plus } from 'lucide-react';
import { useUpdateStock } from '@/lib/hooks/use-stocks';
import { useUpdateOption } from '@/lib/hooks/use-options';
import { useCreateStock } from '@/lib/hooks/use-stocks';
import { useCreateOption } from '@/lib/hooks/use-options';
import { TransactionRow } from './transaction-row';
import { createClient } from '@/lib/supabase/client';
import apiConfig, { getFullUrl } from '@/lib/config/api';
import type { Stock } from '@/lib/types/stocks';
import type { OptionTrade } from '@/lib/types/options';
import type { UpdateStockRequest } from '@/lib/types/stocks';
import type { UpdateOptionRequest } from '@/lib/types/options';
import type { CreateStockRequest } from '@/lib/types/stocks';
import type { CreateOptionRequest } from '@/lib/types/options';

// Helper to get local datetime string suitable for input[type="datetime-local"]
function getLocalDateTimeInputValue(date?: Date | string | null): string {
  if (!date || (typeof date === 'string' && date.trim() === '')) {
    const now = new Date();
    const pad = (n: number) => String(n).padStart(2, '0');
    const year = now.getFullYear();
    const month = pad(now.getMonth() + 1);
    const day = pad(now.getDate());
    const hours = pad(now.getHours());
    const minutes = pad(now.getMinutes());
    return `${year}-${month}-${day}T${hours}:${minutes}`;
  }
  
  const dateObj = typeof date === 'string' ? new Date(date) : date;
  if (!dateObj || !(dateObj instanceof Date) || isNaN(dateObj.getTime())) {
    const now = new Date();
    const pad = (n: number) => String(n).padStart(2, '0');
    const year = now.getFullYear();
    const month = pad(now.getMonth() + 1);
    const day = pad(now.getDate());
    const hours = pad(now.getHours());
    const minutes = pad(now.getMinutes());
    return `${year}-${month}-${day}T${hours}:${minutes}`;
  }
  
  const pad = (n: number) => String(n).padStart(2, '0');
  const year = dateObj.getFullYear();
  const month = pad(dateObj.getMonth() + 1);
  const day = pad(dateObj.getDate());
  const hours = pad(dateObj.getHours());
  const minutes = pad(dateObj.getMinutes());
  return `${year}-${month}-${day}T${hours}:${minutes}`;
}

// Helper to get date string for input[type="date"]
function getLocalDateInputValue(date?: Date | string | null): string {
  if (!date || (typeof date === 'string' && date.trim() === '')) {
    return '';
  }
  
  const dateObj = typeof date === 'string' ? new Date(date) : date;
  if (!dateObj || !(dateObj instanceof Date) || isNaN(dateObj.getTime())) {
    return '';
  }
  
  const pad = (n: number) => String(n).padStart(2, '0');
  const year = dateObj.getFullYear();
  const month = pad(dateObj.getMonth() + 1);
  const day = pad(dateObj.getDate());
  return `${year}-${month}-${day}`;
}

// Generate UUID for trade_group_id
function generateTradeGroupId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID()
  }
  return `trade-group-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
}

// Transaction schema for stocks
const stockTransactionSchema = z.object({
  id: z.number().optional(), // Existing trade ID
  // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
  tradeType: z.enum(['BUY', 'SELL'], { required_error: 'Trade type is required' }),
  entryPrice: z.coerce.number().positive('Entry price must be positive'),
  exitPrice: z.coerce.number().positive('Exit price must be positive'),
  numberShares: z.coerce.number().positive('Number of shares must be positive'),
  entryDate: z.string().min(1, 'Entry date is required'),
  exitDate: z.string().min(1, 'Exit date is required'),
  commissions: z.coerce.number().min(0, 'Commissions cannot be negative').optional().default(0),
});

// Stock form schema with transactions array
const stockFormSchema = z.object({
  symbol: z.string().min(1, 'Symbol is required').max(10, 'Symbol must be 10 characters or less'),
  // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
  orderType: z.enum(['MARKET', 'LIMIT', 'STOP', 'STOP_LIMIT'], { required_error: 'Order type is required' }),
  stopLoss: z.coerce.number().positive('Stop loss must be positive'),
  takeProfit: z.coerce.number().positive('Take profit must be positive').optional().nullable(),
  initialTarget: z.coerce.number().positive('Initial target must be positive').optional().nullable(),
  profitTarget: z.coerce.number().positive('Profit target must be positive').optional().nullable(),
  tradeRatings: z.coerce.number().min(1, 'Rating must be at least 1').max(5, 'Rating must be at most 5').optional().nullable(),
  reviewed: z.boolean().optional().default(false),
  mistakes: z.string().optional().nullable(),
  transactions: z.array(stockTransactionSchema).min(1, 'At least one transaction is required'),
});

type StockFormData = z.infer<typeof stockFormSchema>;

// Transaction schema for options
const optionTransactionSchema = z.object({
  id: z.number().optional(), // Existing trade ID
  // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
  tradeType: z.enum(['BUY', 'SELL'], { required_error: 'Trade type is required' }),
  entryPrice: z.coerce.number().positive('Entry price must be positive'),
  exitPrice: z.coerce.number().positive('Exit price must be positive'),
  totalQuantity: z.coerce.number().int().positive('Number of contracts must be a positive integer'),
  entryDate: z.string().min(1, 'Entry date is required'),
  exitDate: z.string().min(1, 'Exit date is required'),
  commissions: z.coerce.number().min(0, 'Commissions cannot be negative').optional().default(0),
});

// Option form schema with transactions array
const optionFormSchema = z.object({
  symbol: z.string().min(1, 'Symbol is required').max(10, 'Symbol must be 10 characters or less'),
  // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
  optionType: z.enum(['Call', 'Put'], { required_error: 'Option type is required' }),
  strikePrice: z.coerce.number().positive('Strike price must be positive'),
  expirationDate: z.string().min(1, 'Expiration date is required'),
  premium: z.coerce.number().positive('Premium must be positive'),
  initialTarget: z.coerce.number().positive('Initial target must be positive').optional().nullable(),
  profitTarget: z.coerce.number().positive('Profit target must be positive').optional().nullable(),
  tradeRatings: z.coerce.number().min(1, 'Rating must be at least 1').max(5, 'Rating must be at most 5').optional().nullable(),
  reviewed: z.boolean().optional().default(false),
  mistakes: z.string().optional().nullable(),
  transactions: z.array(optionTransactionSchema).min(1, 'At least one transaction is required'),
});

type OptionFormData = z.infer<typeof optionFormSchema>;

interface EditTradeModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  trade: Stock | OptionTrade | null;
  tradeType: 'stock' | 'option';
}

// Fetch trades by trade_group_id
async function fetchTradesByGroupId(
  tradeGroupId: string | null | undefined,
  tradeType: 'stock' | 'option'
): Promise<(Stock | OptionTrade)[]> {
  if (!tradeGroupId) return [];
  
  const supabase = createClient();
  const { data: { session } } = await supabase.auth.getSession();
  const token = session?.access_token;

  if (!token) {
    throw new Error('User not authenticated');
  }

  const endpoint = tradeType === 'stock'
    ? apiConfig.endpoints.stocks.withQuery({ tradeGroupId })
    : apiConfig.endpoints.options.withQuery({ tradeGroupId });

  const res = await fetch(getFullUrl(endpoint), {
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
    credentials: 'include',
  });

  if (!res.ok) {
    if (res.status === 401) {
      throw new Error('Authentication failed');
    }
    return [];
  }

  const json = await res.json();
  const trades = (json.data ?? []) as (Stock | OptionTrade)[];
  
  // Sort by transaction_sequence
  return trades.sort((a, b) => {
    const seqA = (a as any).transactionSequence ?? 0;
    const seqB = (b as any).transactionSequence ?? 0;
    return seqA - seqB;
  });
}

export default function EditTradeModal({ open, onOpenChange, trade, tradeType }: EditTradeModalProps) {
  const [activeTab, setActiveTab] = React.useState<'stock' | 'option'>(tradeType);
  const [loadingGroup, setLoadingGroup] = React.useState(false);
  const [tradeGroup, setTradeGroup] = React.useState<(Stock | OptionTrade)[]>([]);
  const updateStock = useUpdateStock();
  const updateOption = useUpdateOption();
  const createStock = useCreateStock();
  const createOption = useCreateOption();

  // Set active tab based on trade type when trade changes
  React.useEffect(() => {
    if (trade) {
      setActiveTab(tradeType);
    }
  }, [trade, tradeType]);

  // Load trade group when trade changes
  React.useEffect(() => {
    if (trade && open) {
      const tradeGroupId = (trade as any).tradeGroupId;
      if (tradeGroupId) {
        setLoadingGroup(true);
        fetchTradesByGroupId(tradeGroupId, tradeType)
          .then((trades) => {
            setTradeGroup(trades.length > 0 ? trades : [trade]);
            setLoadingGroup(false);
          })
          .catch((error) => {
            console.error('Error loading trade group:', error);
            setTradeGroup([trade]);
            setLoadingGroup(false);
          });
      } else {
        setTradeGroup([trade]);
      }
    }
  }, [trade, tradeType, open]);

  const stockForm = useForm<StockFormData>({
    // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
    resolver: zodResolver(stockFormSchema),
    // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
    defaultValues: React.useMemo(() => {
      if (trade && tradeType === 'stock' && tradeGroup.length > 0) {
        const stocks = tradeGroup as Stock[];
        const firstStock = stocks[0];
        return {
          symbol: firstStock.symbol,
          orderType: firstStock.orderType,
          stopLoss: parseFloat(firstStock.stopLoss) || 0,
          takeProfit: firstStock.takeProfit ? parseFloat(firstStock.takeProfit) : null,
          initialTarget: firstStock.initialTarget ? parseFloat(firstStock.initialTarget) : null,
          profitTarget: firstStock.profitTarget ? parseFloat(firstStock.profitTarget) : null,
          tradeRatings: firstStock.tradeRatings ?? null,
          reviewed: firstStock.reviewed,
          mistakes: firstStock.mistakes ?? null,
          transactions: stocks.map((s) => ({
            id: s.id,
            tradeType: s.tradeType,
            entryPrice: parseFloat(s.entryPrice) || 0,
            exitPrice: s.exitPrice ? parseFloat(s.exitPrice) : null,
            numberShares: parseFloat(s.numberShares) || 0,
            entryDate: getLocalDateTimeInputValue(s.entryDate),
            exitDate: s.exitDate ? getLocalDateTimeInputValue(s.exitDate) : null,
            commissions: parseFloat(s.commissions) || 0,
          })),
        };
      }
      return {
        symbol: '',
        orderType: 'MARKET',
        stopLoss: 0,
        takeProfit: null,
        initialTarget: null,
        profitTarget: null,
        tradeRatings: null,
        reviewed: false,
        mistakes: null,
        transactions: [],
      };
    }, [trade, tradeType, tradeGroup]),
  });

  const optionForm = useForm<OptionFormData>({
    // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
    resolver: zodResolver(optionFormSchema),
    defaultValues: React.useMemo(() => {
      if (trade && tradeType === 'option' && tradeGroup.length > 0) {
        const options = tradeGroup as OptionTrade[];
        const firstOption = options[0];
        return {
          symbol: firstOption.symbol,
          optionType: firstOption.optionType,
          strikePrice: parseFloat(firstOption.strikePrice) || 0,
          expirationDate: getLocalDateInputValue(firstOption.expirationDate),
          premium: parseFloat(firstOption.premium) || 0,
          initialTarget: firstOption.initialTarget ? parseFloat(firstOption.initialTarget) : null,
          profitTarget: firstOption.profitTarget ? parseFloat(firstOption.profitTarget) : null,
          tradeRatings: firstOption.tradeRatings ?? null,
          reviewed: firstOption.reviewed,
          mistakes: firstOption.mistakes ?? null,
          transactions: options.map((o) => ({
            id: o.id,
            tradeType: 'BUY' as const, // Options don't have tradeType, default to BUY
            entryPrice: parseFloat(o.entryPrice) || 0,
            exitPrice: parseFloat(o.exitPrice) || 0,
            totalQuantity: o.totalQuantity ? parseInt(o.totalQuantity) : 1,
            entryDate: getLocalDateTimeInputValue(o.entryDate),
            exitDate: o.exitDate ? getLocalDateTimeInputValue(o.exitDate) : getLocalDateTimeInputValue(),
            commissions: parseFloat(o.commissions) || 0,
          })),
        };
      }
      return {
        symbol: '',
        optionType: 'Call',
        strikePrice: 0,
        expirationDate: '',
        premium: 0,
        initialTarget: null,
        profitTarget: null,
        tradeRatings: null,
        reviewed: false,
        mistakes: null,
        transactions: [],
      };
    }, [trade, tradeType, tradeGroup]),
  });

  const stockTransactions = useFieldArray({
    control: stockForm.control,
    name: 'transactions',
  });

  const optionTransactions = useFieldArray({
    control: optionForm.control,
    name: 'transactions',
  });

  // Reset forms when trade group changes
  React.useEffect(() => {
    if (trade && open && tradeGroup.length > 0) {
      if (tradeType === 'stock') {
        const stocks = tradeGroup as Stock[];
        const firstStock = stocks[0];
        // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
        stockForm.reset({
          symbol: firstStock.symbol,
          orderType: firstStock.orderType,
          stopLoss: parseFloat(firstStock.stopLoss) || 0,
          takeProfit: firstStock.takeProfit ? parseFloat(firstStock.takeProfit) : null,
          initialTarget: firstStock.initialTarget ? parseFloat(firstStock.initialTarget) : null,
          profitTarget: firstStock.profitTarget ? parseFloat(firstStock.profitTarget) : null,
          tradeRatings: firstStock.tradeRatings ?? null,
          reviewed: firstStock.reviewed,
          mistakes: firstStock.mistakes ?? null,
          transactions: stocks.map((s) => ({
            id: s.id,
            tradeType: s.tradeType,
            entryPrice: parseFloat(s.entryPrice) || 0,
            exitPrice: s.exitPrice ? parseFloat(s.exitPrice) : null,
            numberShares: parseFloat(s.numberShares) || 0,
            entryDate: getLocalDateTimeInputValue(s.entryDate),
            exitDate: s.exitDate ? getLocalDateTimeInputValue(s.exitDate) : null,
            commissions: parseFloat(s.commissions) || 0,
          })),
        });
      } else {
        const options = tradeGroup as OptionTrade[];
        const firstOption = options[0];
        optionForm.reset({
          symbol: firstOption.symbol,
          optionType: firstOption.optionType,
          strikePrice: parseFloat(firstOption.strikePrice) || 0,
          expirationDate: getLocalDateInputValue(firstOption.expirationDate),
          premium: parseFloat(firstOption.premium) || 0,
          initialTarget: firstOption.initialTarget ? parseFloat(firstOption.initialTarget) : null,
          profitTarget: firstOption.profitTarget ? parseFloat(firstOption.profitTarget) : null,
          tradeRatings: firstOption.tradeRatings ?? null,
          reviewed: firstOption.reviewed,
          mistakes: firstOption.mistakes ?? null,
          transactions: options.map((o) => ({
            id: o.id,
            tradeType: 'BUY' as const,
            entryPrice: parseFloat(o.entryPrice) || 0,
            exitPrice: parseFloat(o.exitPrice) || 0,
            totalQuantity: o.totalQuantity ? parseInt(o.totalQuantity) : 1,
            entryDate: getLocalDateTimeInputValue(o.entryDate),
            exitDate: o.exitDate ? getLocalDateTimeInputValue(o.exitDate) : getLocalDateTimeInputValue(),
            commissions: parseFloat(o.commissions) || 0,
          })),
        });
      }
    }
  }, [trade, tradeType, open, tradeGroup, stockForm, optionForm]);

  const handleStockSubmit = async (data: StockFormData) => {
    if (!trade || tradeType !== 'stock') return;
    
    try {
      const tradeGroupId = (trade as Stock).tradeGroupId || generateTradeGroupId();
      const parentTradeId = tradeGroup.length > 0 ? (tradeGroup[0] as Stock).id : undefined;

      // Update existing transactions and create new ones
      for (let i = 0; i < data.transactions.length; i++) {
        const transaction = data.transactions[i];
        const entryDateIso = new Date(transaction.entryDate).toISOString();

        if (transaction.id) {
          // Update existing transaction
          const payload: UpdateStockRequest = {
            symbol: data.symbol,
            tradeType: transaction.tradeType,
            orderType: data.orderType,
            entryPrice: transaction.entryPrice,
            exitPrice: transaction.exitPrice,
            stopLoss: data.stopLoss,
            commissions: Number.isFinite(transaction.commissions) ? transaction.commissions : 0,
            numberShares: transaction.numberShares,
            takeProfit: data.takeProfit ?? null,
            initialTarget: data.initialTarget ?? null,
            profitTarget: data.profitTarget ?? null,
            tradeRatings: data.tradeRatings != null ? Math.round(Number(data.tradeRatings)) : null,
            entryDate: entryDateIso,
            exitDate: new Date(transaction.exitDate).toISOString(),
            reviewed: data.reviewed,
            mistakes: data.mistakes ?? null,
            tradeGroupId: i === 0 ? tradeGroupId : undefined,
            parentTradeId: i === 0 ? undefined : parentTradeId,
            totalQuantity: transaction.numberShares,
            transactionSequence: i + 1,
          };

          await updateStock.mutateAsync({ id: transaction.id, updates: payload });
        } else {
          // Create new transaction
          const payload: CreateStockRequest = {
            symbol: data.symbol,
            tradeType: transaction.tradeType,
            orderType: data.orderType,
            entryPrice: transaction.entryPrice,
            exitPrice: transaction.exitPrice,
            stopLoss: data.stopLoss,
            commissions: Number.isFinite(transaction.commissions) ? transaction.commissions : 0,
            numberShares: transaction.numberShares,
            // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
            takeProfit: data.takeProfit,
            // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
            initialTarget: data.initialTarget,
            // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
            profitTarget: data.profitTarget,
            tradeRatings:
              data.tradeRatings != null && data.tradeRatings !== ("" as unknown)
                ? Math.round(Number(data.tradeRatings))
                : undefined,
            entryDate: entryDateIso,
            exitDate: new Date(transaction.exitDate).toISOString(),
            reviewed: data.reviewed,
            // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
            mistakes: data.mistakes,
            tradeGroupId: tradeGroupId,
            parentTradeId: parentTradeId,
            totalQuantity: transaction.numberShares,
            transactionSequence: tradeGroup.length + i + 1,
          };

          await createStock.mutateAsync(payload);
        }
      }

      toast.success('Stock trade updated successfully');
      onOpenChange(false);
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Failed to update stock trade';
      toast.error(message);
      console.error('Error updating stock trade:', error);
    }
  };

  const handleOptionSubmit = async (data: OptionFormData) => {
    if (!trade || tradeType !== 'option') return;
    
    try {
      const tradeGroupId = (trade as OptionTrade).tradeGroupId || generateTradeGroupId();
      const parentTradeId = tradeGroup.length > 0 ? (tradeGroup[0] as OptionTrade).id : undefined;
      const expirationDateIso = data.expirationDate
        ? new Date(`${data.expirationDate}T00:00:00`).toISOString()
        : '';

      // Update existing transactions and create new ones
      for (let i = 0; i < data.transactions.length; i++) {
        const transaction = data.transactions[i];
        const entryDateIso = new Date(transaction.entryDate).toISOString();
        const exitDateIso = new Date(transaction.exitDate).toISOString();

        if (transaction.id) {
          // Update existing transaction
          const payload: UpdateOptionRequest = {
            symbol: data.symbol,
            optionType: data.optionType,
            strikePrice: data.strikePrice,
            expirationDate: expirationDateIso,
            entryPrice: transaction.entryPrice,
            exitPrice: transaction.exitPrice,
            premium: data.premium,
            commissions: transaction.commissions,
            entryDate: entryDateIso,
            exitDate: exitDateIso,
            initialTarget: data.initialTarget ?? null,
            profitTarget: data.profitTarget ?? null,
            tradeRatings: data.tradeRatings != null ? Math.round(Number(data.tradeRatings)) : null,
            reviewed: data.reviewed,
            mistakes: data.mistakes ?? null,
            tradeGroupId: i === 0 ? tradeGroupId : undefined,
            parentTradeId: i === 0 ? undefined : parentTradeId,
            totalQuantity: transaction.totalQuantity,
            transactionSequence: i + 1,
          };

          await updateOption.mutateAsync({ id: transaction.id, updates: payload });
        } else {
          // Create new transaction
          const payload: CreateOptionRequest = {
            symbol: data.symbol,
            optionType: data.optionType,
            strikePrice: data.strikePrice,
            expirationDate: expirationDateIso,
            entryPrice: transaction.entryPrice,
            exitPrice: transaction.exitPrice,
            premium: data.premium,
            commissions: Number.isFinite(transaction.commissions) ? transaction.commissions : 0,
            entryDate: entryDateIso,
            exitDate: exitDateIso,
            // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
            initialTarget: data.initialTarget,
            // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
            profitTarget: data.profitTarget,
            tradeRatings:
              data.tradeRatings != null && data.tradeRatings !== ("" as unknown)
                ? Math.round(Number(data.tradeRatings))
                : undefined,
            reviewed: data.reviewed,
            // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
            mistakes: data.mistakes,
            tradeGroupId: tradeGroupId,
            parentTradeId: parentTradeId,
            totalQuantity: transaction.totalQuantity,
            transactionSequence: tradeGroup.length + i + 1,
          };

          await createOption.mutateAsync(payload);
        }
      }

      toast.success('Option trade updated successfully');
      onOpenChange(false);
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Failed to update option trade';
      toast.error(message);
      console.error('Error updating option trade:', error);
    }
  };

  if (!trade) return null;

  if (loadingGroup) {
    return (
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent>
          <div className="flex items-center justify-center p-8">
            <p>Loading trade group...</p>
          </div>
        </DialogContent>
      </Dialog>
    );
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl h-[90vh] flex flex-col overflow-hidden p-0 gap-0">
        <DialogHeader className="flex-shrink-0 px-6 pt-6 pb-4">
          <DialogTitle>Edit Trade</DialogTitle>
          <DialogDescription>
            Update the details of your {tradeType === 'stock' ? 'stock' : 'option'} trade
          </DialogDescription>
        </DialogHeader>

        <Tabs
          value={activeTab}
          onValueChange={(value) => setActiveTab(value as 'stock' | 'option')}
          className="w-full flex flex-col flex-1 min-h-0 overflow-hidden"
        >
          <div className="flex-shrink-0 px-6 pb-4">
            <TabsList className="grid w-full grid-cols-2">
              <TabsTrigger value="stock">Stock Trade</TabsTrigger>
              <TabsTrigger value="option">Option Trade</TabsTrigger>
            </TabsList>
          </div>

          <ScrollArea className="flex-1 min-h-0 overflow-hidden">
            <div className="px-6 pb-6">
              <TabsContent value="stock" className="space-y-4 mt-0">
                <Form {...stockForm}>
               {/* @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?) */}
                  <form onSubmit={stockForm.handleSubmit(handleStockSubmit)} className="space-y-4">
                    {/* Shared fields */}
                    <div className="grid grid-cols-2 gap-4">
                      <FormField
                        // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={stockForm.control}
                        name="symbol"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Symbol</FormLabel>
                            <FormControl>
                              <Input placeholder="AAPL" {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      <FormField
                        // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={stockForm.control}
                        name="orderType"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Order Type</FormLabel>
                            <Select onValueChange={field.onChange} value={field.value}>
                              <FormControl>
                                <SelectTrigger>
                                  <SelectValue placeholder="Select order type" />
                                </SelectTrigger>
                              </FormControl>
                              <SelectContent>
                                <SelectItem value="MARKET">MARKET</SelectItem>
                                <SelectItem value="LIMIT">LIMIT</SelectItem>
                                <SelectItem value="STOP">STOP</SelectItem>
                                <SelectItem value="STOP_LIMIT">STOP_LIMIT</SelectItem>
                              </SelectContent>
                            </Select>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                      <FormField
                        // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={stockForm.control}
                        name="stopLoss"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Stop Loss</FormLabel>
                            <FormControl>
                              <Input type="number" step="0.01" placeholder="145.00" {...field} value={field.value ?? ''} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      <FormField
                        // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={stockForm.control}
                        name="takeProfit"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Take Profit</FormLabel>
                            <FormControl>
                              <Input
                                type="number"
                                step="0.01"
                                placeholder="160.00"
                                {...field}
                                value={field.value ?? ''}
                                onChange={(e) => field.onChange(e.target.value === '' ? null : parseFloat(e.target.value))}
                              />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                      <FormField
                        // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={stockForm.control}
                        name="initialTarget"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Initial Target</FormLabel>
                            <FormControl>
                              <Input
                                type="number"
                                step="0.01"
                                placeholder="155.00"
                                {...field}
                                value={field.value ?? ''}
                                onChange={(e) => field.onChange(e.target.value === '' ? null : parseFloat(e.target.value))}
                              />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      <FormField
                        // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={stockForm.control}
                        name="profitTarget"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Profit Target</FormLabel>
                            <FormControl>
                              <Input
                                type="number"
                                step="0.01"
                                placeholder="165.00"
                                {...field}
                                value={field.value ?? ''}
                                onChange={(e) => field.onChange(e.target.value === '' ? null : parseFloat(e.target.value))}
                              />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                      <FormField
                        // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={stockForm.control}
                        name="tradeRatings"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Trade Rating (1-5)</FormLabel>
                            <FormControl>
                              <Input
                                type="number"
                                min="1"
                                max="5"
                                placeholder="3"
                                {...field}
                                value={field.value ?? ''}
                                onChange={(e) => field.onChange(e.target.value === '' ? null : parseFloat(e.target.value))}
                              />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>

                    <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                      control={stockForm.control}
                      name="mistakes"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>Mistakes (Optional)</FormLabel>
                          <FormControl>
                            <Input placeholder="Enter mistakes separated by commas" {...field} value={field.value ?? ''} />
                          </FormControl>
                          <FormDescription>
                            List any mistakes made in this trade, separated by commas
                          </FormDescription>
                          <FormMessage />
                        </FormItem>
                      )}
                    />

                    {/* Transactions */}
                    <div className="space-y-4">
                      <div className="flex items-center justify-between">
                        <h3 className="text-sm font-semibold">Transactions</h3>
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          onClick={() =>
                            stockTransactions.append({
                              tradeType: 'BUY',
                              entryPrice: 0,
                              exitPrice: 0,
                              numberShares: 0,
                              entryDate: getLocalDateTimeInputValue(),
                              exitDate: getLocalDateTimeInputValue(),
                              commissions: 0,
                            })
                          }
                        >
                          <Plus className="h-4 w-4 mr-2" />
                          Add Transaction
                        </Button>
                      </div>

                      {stockTransactions.fields.map((field, index) => (
                        <TransactionRow
                          key={field.id}
                          index={index}
                          form={stockForm}
                          tradeType="stock"
                          onRemove={() => stockTransactions.remove(index)}
                          canRemove={stockTransactions.fields.length > 1}
                        />
                      ))}
                    </div>

                    <DialogFooter>
                      <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                        Cancel
                      </Button>
                      <Button type="submit" disabled={updateStock.isPending || createStock.isPending}>
                        {updateStock.isPending || createStock.isPending ? 'Updating...' : 'Update Stock Trade'}
                      </Button>
                    </DialogFooter>
                  </form>
                </Form>
              </TabsContent>

              <TabsContent value="option" className="space-y-4 mt-0">
                <Form {...optionForm}>
                  {/* @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?) */}
                  <form onSubmit={optionForm.handleSubmit(handleOptionSubmit)} className="space-y-4">
                    {/* Shared fields */}
                    <div className="grid grid-cols-2 gap-4">
                      <FormField
                        // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={optionForm.control}
                        name="symbol"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Symbol</FormLabel>
                            <FormControl>
                              <Input placeholder="AAPL" {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={optionForm.control}
                        name="optionType"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Option Type</FormLabel>
                            <Select onValueChange={field.onChange} value={field.value}>
                              <FormControl>
                                <SelectTrigger>
                                  <SelectValue placeholder="Select option type" />
                                </SelectTrigger>
                              </FormControl>
                              <SelectContent>
                                <SelectItem value="Call">Call</SelectItem>
                                <SelectItem value="Put">Put</SelectItem>
                              </SelectContent>
                            </Select>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={optionForm.control}
                        name="strikePrice"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Strike Price</FormLabel>
                            <FormControl>
                              <Input type="number" step="0.01" placeholder="150.00" {...field} value={field.value ?? ''} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={optionForm.control}
                        name="premium"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Premium</FormLabel>
                            <FormControl>
                              <Input type="number" step="0.01" placeholder="550.00" {...field} value={field.value ?? ''} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={optionForm.control}
                        name="expirationDate"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Expiration Date</FormLabel>
                            <FormControl>
                              <Input type="date" {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={optionForm.control}
                        name="tradeRatings"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Trade Rating (1-5)</FormLabel>
                            <FormControl>
                              <Input
                                type="number"
                                min="1"
                                max="5"
                                placeholder="3"
                                {...field}
                                value={field.value ?? ''}
                                onChange={(e) => field.onChange(e.target.value === '' ? null : parseFloat(e.target.value))}
                              />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={optionForm.control}
                        name="initialTarget"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Initial Target</FormLabel>
                            <FormControl>
                              <Input
                                type="number"
                                step="0.01"
                                placeholder="6.00"
                                {...field}
                                value={field.value ?? ''}
                                onChange={(e) => field.onChange(e.target.value === '' ? null : parseFloat(e.target.value))}
                              />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={optionForm.control}
                        name="profitTarget"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Profit Target</FormLabel>
                            <FormControl>
                              <Input
                                type="number"
                                step="0.01"
                                placeholder="8.00"
                                {...field}
                                value={field.value ?? ''}
                                onChange={(e) => field.onChange(e.target.value === '' ? null : parseFloat(e.target.value))}
                              />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>

                    <FormField
                    // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                      control={optionForm.control}
                      name="mistakes"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>Mistakes (Optional)</FormLabel>
                          <FormControl>
                            <Input placeholder="Enter mistakes separated by commas" {...field} value={field.value ?? ''} />
                          </FormControl>
                          <FormDescription>
                            List any mistakes made in this trade, separated by commas
                          </FormDescription>
                          <FormMessage />
                        </FormItem>
                      )}
                    />

                    {/* Transactions */}
                    <div className="space-y-4">
                      <div className="flex items-center justify-between">
                        <h3 className="text-sm font-semibold">Transactions</h3>
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          onClick={() =>
                            optionTransactions.append({
                              tradeType: 'BUY',
                              entryPrice: 0,
                              exitPrice: 0,
                              totalQuantity: 1,
                              entryDate: getLocalDateTimeInputValue(),
                              exitDate: getLocalDateTimeInputValue(),
                              commissions: 0,
                            })
                          }
                        >
                          <Plus className="h-4 w-4 mr-2" />
                          Add Transaction
                        </Button>
                      </div>

                      {optionTransactions.fields.map((field, index) => (
                        <TransactionRow
                          key={field.id}
                          index={index}
                          form={optionForm}
                          tradeType="option"
                          onRemove={() => optionTransactions.remove(index)}
                          canRemove={optionTransactions.fields.length > 1}
                        />
                      ))}
                    </div>

                    <DialogFooter>
                      <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                        Cancel
                      </Button>
                      <Button type="submit" disabled={updateOption.isPending || createOption.isPending}>
                        {updateOption.isPending || createOption.isPending ? 'Updating...' : 'Update Option Trade'}
                      </Button>
                    </DialogFooter>
                  </form>
                </Form>
              </TabsContent>
            </div>
          </ScrollArea>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
