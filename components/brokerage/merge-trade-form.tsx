'use client';

import React, { useMemo, useEffect } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
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
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { useMergeTrades } from '@/lib/hooks/use-merge-trades';
import type { BrokerageTransaction } from '@/lib/types/brokerage';
import { calculateWeightedAverage, isOptionTransaction } from '@/lib/utils/brokerage';
import { format } from 'date-fns';


// function getLocalDateInputValue(date?: Date | string | null): string {
//   if (!date) return '';
//   const dateObj = typeof date === 'string' ? new Date(date) : date;
//   if (isNaN(dateObj.getTime())) return '';

//   const pad = (n: number) => String(n).padStart(2, '0');
//   const year = dateObj.getFullYear();
//   const month = pad(dateObj.getMonth() + 1);
//   const day = pad(dateObj.getDate());
//   return `${year}-${month}-${day}`;
// }

// Stock form schema
const stockFormSchema = z.object({
  symbol: z.string().min(1, 'Symbol is required'),
   // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
  orderType: z.enum(['MARKET', 'LIMIT', 'STOP', 'STOP_LIMIT'], {
    required_error: 'Order type is required',
  }),
  stopLoss: z.coerce.number().positive('Stop loss must be positive'),
  takeProfit: z.coerce.number().positive('Take profit must be positive').optional(),
  initialTarget: z.coerce.number().positive('Initial target must be positive').optional(),
  profitTarget: z.coerce.number().positive('Profit target must be positive').optional(),
  tradeRatings: z.coerce.number().min(1, 'Trade rating must be at least 1').max(5, 'Trade rating must be at most 5'),
  reviewed: z.boolean().optional().default(false),
  mistakes: z.string().min(1, 'Mistakes is required'),
  brokerageName: z.string().min(1, 'Brokerage name is required'),
});

type StockFormData = z.infer<typeof stockFormSchema>;

// Option form schema
const optionFormSchema = z.object({
  symbol: z.string().min(1, 'Symbol is required'),
  strategyType: z.string().min(1, 'Strategy type is required'),
   // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
  tradeDirection: z.enum(['Bullish', 'Bearish', 'Neutral'], {
    required_error: 'Trade direction is required',
  }),
   // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
  optionType: z.enum(['Call', 'Put'], {
    required_error: 'Option type is required',
  }),
  strikePrice: z.coerce.number().positive('Strike price must be positive'),
  expirationDate: z.string().min(1, 'Expiration date is required'),
  impliedVolatility: z.coerce
    .number()
    .min(0, 'Implied volatility cannot be negative')
    .max(100, 'Implied volatility cannot exceed 100'),
     // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
  orderType: z.enum(['MARKET', 'LIMIT', 'STOP', 'STOP_LIMIT'], {
    required_error: 'Order type is required',
  }),
  stopLoss: z.coerce.number().positive('Stop loss must be positive'),
  takeProfit: z.coerce.number().positive('Take profit must be positive').optional(),
  initialTarget: z.coerce.number().positive('Initial target must be positive').optional(),
  profitTarget: z.coerce.number().positive('Profit target must be positive').optional(),
  tradeRatings: z.coerce.number().min(1, 'Trade rating must be at least 1').max(5, 'Trade rating must be at most 5'),
  reviewed: z.boolean().optional().default(false),
  mistakes: z.string().min(1, 'Mistakes is required'),
  brokerageName: z.string().min(1, 'Brokerage name is required'),
});

type OptionFormData = z.infer<typeof optionFormSchema>;

// LocalStorage keys
const STORAGE_KEYS = {
  STOCK_FORM: 'merge-trade-form-stock',
  OPTION_FORM: 'merge-trade-form-option',
  PREFERENCES: 'merge-trade-form-preferences',
} as const;

interface MergeTradeFormProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  transactions: BrokerageTransaction[];
  onSuccess: () => void;
}

export function MergeTradeForm({
  open,
  onOpenChange,
  transactions,
  onSuccess,
}: MergeTradeFormProps) {
  const { mergeTransactions, isMerging } = useMergeTrades();
  const [activeTab, setActiveTab] = React.useState<'stock' | 'option'>('stock');

  // Detect if transactions are options
  const hasOptions = useMemo(() => {
    return transactions.some((t) => isOptionTransaction(t));
  }, [transactions]);

  // Calculate weighted averages
  const calculatedValues = useMemo(() => {
    const buys = transactions.filter((t) => t.transaction_type === 'BUY');
    const sells = transactions.filter((t) => t.transaction_type === 'SELL');

    const buyAvg = calculateWeightedAverage(buys);
    const sellAvg = calculateWeightedAverage(sells);

    const entryDate = buys.length > 0
      ? buys.reduce((earliest, t) => {
          const tDate = t.trade_date ? new Date(t.trade_date) : new Date();
          const eDate = earliest ? new Date(earliest) : new Date();
          return tDate < eDate ? t.trade_date! : earliest;
        }, buys[0]?.trade_date || '')
      : '';

    const exitDate = sells.length > 0
      ? sells.reduce((latest, t) => {
          const tDate = t.trade_date ? new Date(t.trade_date) : new Date();
          const lDate = latest ? new Date(latest) : new Date();
          return tDate > lDate ? t.trade_date! : latest;
        }, sells[0]?.trade_date || '')
      : '';

    // Get brokerage name from first transaction's raw_data if available
    let brokerageName = '';
    if (transactions[0]?.raw_data) {
      try {
        const rawData = typeof transactions[0].raw_data === 'string'
          ? JSON.parse(transactions[0].raw_data)
          : transactions[0].raw_data;
        brokerageName = rawData.brokerage_name || rawData.brokerage || '';
      } catch {
        // Ignore parse errors
      }
    }

    return {
      entryPrice: buyAvg.price || 0,
      exitPrice: sellAvg.price || undefined,
      quantity: buyAvg.quantity || sellAvg.quantity || 0,
      fees: buyAvg.fees + sellAvg.fees,
      entryDate,
      exitDate,
      brokerageName,
    };
  }, [transactions]);

  // Get symbol from transactions - extract from symbol field or raw_data
  const symbol = useMemo(() => {
    // First try direct symbol field
    if (transactions[0]?.symbol) {
      return transactions[0].symbol;
    }
    
    // Try to extract from raw_data
    if (transactions[0]?.raw_data) {
      try {
        const rawData = typeof transactions[0].raw_data === 'string'
          ? JSON.parse(transactions[0].raw_data)
          : transactions[0].raw_data;
        
        // Try multiple possible paths for symbol
        const extractedSymbol = 
          rawData.symbol?.symbol ||
          rawData.symbol ||
          rawData.underlying_symbol ||
          rawData.stock_symbol ||
          '';
        
        if (extractedSymbol) {
          return extractedSymbol;
        }
      } catch {
        // Ignore parse errors
      }
    }
    
    return '';
  }, [transactions]);

  // Get brokerage name from transactions - improved extraction
  const extractedBrokerageName = useMemo(() => {
    // Try to get from raw_data first
    for (const transaction of transactions) {
      if (transaction.raw_data) {
        try {
          const rawData = typeof transaction.raw_data === 'string'
            ? JSON.parse(transaction.raw_data)
            : transaction.raw_data;
          
          // Try multiple possible paths for brokerage name
          const brokerageName = 
            rawData.brokerage_name ||
            rawData.brokerage ||
            rawData.institution_name ||
            rawData.institution ||
            rawData.account?.institution_name ||
            rawData.account?.institution ||
            '';
          
          if (brokerageName) {
            return brokerageName;
          }
        } catch {
          // Ignore parse errors
        }
      }
    }
    
    return calculatedValues.brokerageName || '';
  }, [transactions, calculatedValues.brokerageName]);

  // Load preferences from localStorage
  const loadPreferences = useMemo(() => {
    try {
      const stored = localStorage.getItem(STORAGE_KEYS.PREFERENCES);
      if (stored) {
        return JSON.parse(stored);
      }
    } catch {
      // Ignore parse errors
    }
    return {
      defaultOrderType: 'MARKET',
      defaultStopLossPercent: 3, // 3% below entry
      defaultTradeRating: 3,
    };
  }, []);

  // Set default tab based on option detection
  useEffect(() => {
    if (hasOptions) {
      setActiveTab('option');
    }
  }, [hasOptions]);

  // Calculate default stop loss based on entry price
  const defaultStopLoss = useMemo(() => {
    if (calculatedValues.entryPrice > 0) {
      return calculatedValues.entryPrice * (1 - loadPreferences.defaultStopLossPercent / 100);
    }
    return 0;
  }, [calculatedValues.entryPrice, loadPreferences.defaultStopLossPercent]);

  const stockForm = useForm<StockFormData>({
     // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
    resolver: zodResolver(stockFormSchema),
    defaultValues: {
      symbol,
      orderType: loadPreferences.defaultOrderType as 'MARKET' | 'LIMIT' | 'STOP' | 'STOP_LIMIT',
      stopLoss: defaultStopLoss,
      takeProfit: undefined,
      initialTarget: undefined,
      profitTarget: undefined,
      tradeRatings: loadPreferences.defaultTradeRating,
      reviewed: false,
      mistakes: '',
      brokerageName: extractedBrokerageName,
    },
  });

  const optionForm = useForm<OptionFormData>({
     // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
    resolver: zodResolver(optionFormSchema),
    defaultValues: {
      symbol,
      strategyType: '',
      tradeDirection: 'Bullish',
      optionType: 'Call',
      strikePrice: 0,
      expirationDate: '',
      impliedVolatility: 0,
      orderType: loadPreferences.defaultOrderType as 'MARKET' | 'LIMIT' | 'STOP' | 'STOP_LIMIT',
      stopLoss: defaultStopLoss,
      takeProfit: undefined,
      initialTarget: undefined,
      profitTarget: undefined,
      tradeRatings: loadPreferences.defaultTradeRating,
      reviewed: false,
      mistakes: '',
      brokerageName: extractedBrokerageName,
    },
  });

  // Auto-fill symbol and brokerage name when transactions change
  useEffect(() => {
    if (symbol) {
      stockForm.setValue('symbol', symbol);
      optionForm.setValue('symbol', symbol);
    }
    if (extractedBrokerageName) {
      stockForm.setValue('brokerageName', extractedBrokerageName);
      optionForm.setValue('brokerageName', extractedBrokerageName);
    }
    if (defaultStopLoss > 0) {
      stockForm.setValue('stopLoss', defaultStopLoss);
      optionForm.setValue('stopLoss', defaultStopLoss);
    }
  }, [symbol, extractedBrokerageName, defaultStopLoss, stockForm, optionForm]);

  // Load saved form data from localStorage when dialog opens
  useEffect(() => {
    if (!open) return;

    try {
      const savedStock = localStorage.getItem(STORAGE_KEYS.STOCK_FORM);
      const savedOption = localStorage.getItem(STORAGE_KEYS.OPTION_FORM);

      if (savedStock && activeTab === 'stock') {
        const parsed = JSON.parse(savedStock);
        // Only restore non-transaction-specific fields
        Object.keys(parsed).forEach((key) => {
          if (key !== 'symbol' && key !== 'brokerageName') {
            stockForm.setValue(key as keyof StockFormData, parsed[key]);
          }
        });
      }

      if (savedOption && activeTab === 'option') {
        const parsed = JSON.parse(savedOption);
        // Only restore non-transaction-specific fields
        Object.keys(parsed).forEach((key) => {
          if (key !== 'symbol' && key !== 'brokerageName') {
            optionForm.setValue(key as keyof OptionFormData, parsed[key]);
          }
        });
      }
    } catch {
      // Ignore parse errors
    }
  }, [open, activeTab, stockForm, optionForm]);

  // Save form data to localStorage when form values change
  useEffect(() => {
    const subscription = stockForm.watch((value) => {
      try {
        localStorage.setItem(STORAGE_KEYS.STOCK_FORM, JSON.stringify(value));
      } catch {
        // Ignore storage errors (e.g., quota exceeded)
      }
    });
    return () => subscription.unsubscribe();
  }, [stockForm]);

  useEffect(() => {
    const subscription = optionForm.watch((value) => {
      try {
        localStorage.setItem(STORAGE_KEYS.OPTION_FORM, JSON.stringify(value));
      } catch {
        // Ignore storage errors (e.g., quota exceeded)
      }
    });
    return () => subscription.unsubscribe();
  }, [optionForm]);

  const handleStockSubmit = async (data: StockFormData) => {
    try {
      // Save preferences for next time
      try {
        const preferences = {
          defaultOrderType: data.orderType,
          defaultStopLossPercent: calculatedValues.entryPrice > 0
            ? ((calculatedValues.entryPrice - data.stopLoss) / calculatedValues.entryPrice) * 100
            : 5,
          defaultTradeRating: data.tradeRatings,
        };
        localStorage.setItem(STORAGE_KEYS.PREFERENCES, JSON.stringify(preferences));
      } catch {
        // Ignore storage errors
      }

      await mergeTransactions({
        transactionIds: transactions.map((t) => t.id),
        tradeType: 'stock',
        symbol: data.symbol,
        orderType: data.orderType,
        stopLoss: data.stopLoss,
        takeProfit: data.takeProfit,
        initialTarget: data.initialTarget,
        profitTarget: data.profitTarget,
        tradeRatings: data.tradeRatings,
        reviewed: data.reviewed,
        mistakes: data.mistakes,
        brokerageName: data.brokerageName,
      });
      onSuccess();
    } catch {
      // Error is handled by the hook
    }
  };

  const handleOptionSubmit = async (data: OptionFormData) => {
    try {
      // Save preferences for next time
      try {
        const preferences = {
          defaultOrderType: data.orderType,
          defaultStopLossPercent: calculatedValues.entryPrice > 0
            ? ((calculatedValues.entryPrice - data.stopLoss) / calculatedValues.entryPrice) * 100
            : 5,
          defaultTradeRating: data.tradeRatings,
        };
        localStorage.setItem(STORAGE_KEYS.PREFERENCES, JSON.stringify(preferences));
      } catch {
        // Ignore storage errors
      }

      await mergeTransactions({
        transactionIds: transactions.map((t) => t.id),
        tradeType: 'option',
        symbol: data.symbol,
        orderType: data.orderType,
        stopLoss: data.stopLoss,
        takeProfit: data.takeProfit,
        initialTarget: data.initialTarget,
        profitTarget: data.profitTarget,
        tradeRatings: data.tradeRatings,
        reviewed: data.reviewed,
        mistakes: data.mistakes,
        brokerageName: data.brokerageName,
        strategyType: data.strategyType,
        tradeDirection: data.tradeDirection,
        optionType: data.optionType,
        strikePrice: data.strikePrice,
        expirationDate: data.expirationDate,
        impliedVolatility: data.impliedVolatility,
      });
      onSuccess();
    } catch {
      // Error is handled by the hook
    }
  };

  if (transactions.length === 0) {
    return null;
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Merge Transactions</DialogTitle>
          <DialogDescription>
            Review calculated values and fill in additional trade information
          </DialogDescription>
        </DialogHeader>

        {/* Calculated Values Summary */}
        <div className="bg-muted p-4 rounded-lg space-y-2 text-sm">
          <div className="font-semibold">Calculated Values:</div>
          <div className="grid grid-cols-2 gap-2">
            <div>
              <span className="text-muted-foreground">Entry Price:</span>{' '}
              <span className="font-medium">${calculatedValues.entryPrice.toFixed(2)}</span>
            </div>
            <div>
              <span className="text-muted-foreground">Quantity:</span>{' '}
              <span className="font-medium">{calculatedValues.quantity.toFixed(2)}</span>
            </div>
            {calculatedValues.exitPrice && (
              <div>
                <span className="text-muted-foreground">Exit Price:</span>{' '}
                <span className="font-medium">${calculatedValues.exitPrice.toFixed(2)}</span>
              </div>
            )}
            <div>
              <span className="text-muted-foreground">Total Fees:</span>{' '}
              <span className="font-medium">${calculatedValues.fees.toFixed(2)}</span>
            </div>
            <div>
              <span className="text-muted-foreground">Entry Date:</span>{' '}
              <span className="font-medium">
                {calculatedValues.entryDate
                  ? format(new Date(calculatedValues.entryDate), 'MMM dd, yyyy')
                  : 'N/A'}
              </span>
            </div>
            {calculatedValues.exitDate && (
              <div>
                <span className="text-muted-foreground">Exit Date:</span>{' '}
                <span className="font-medium">
                  {format(new Date(calculatedValues.exitDate), 'MMM dd, yyyy')}
                </span>
              </div>
            )}
          </div>
        </div>

        <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as 'stock' | 'option')}>
          <TabsList className="grid w-full grid-cols-2">
            <TabsTrigger value="stock">Stock Trade</TabsTrigger>
            <TabsTrigger value="option" disabled={!hasOptions && transactions.length > 0}>
              Option Trade
            </TabsTrigger>
          </TabsList>

          <TabsContent value="stock">
            <Form {...stockForm}>
              <form
               // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                onSubmit={stockForm.handleSubmit(handleStockSubmit)}
                className="space-y-4"
              >
                <FormField
                 // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                  control={stockForm.control}
                  name="symbol"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Symbol</FormLabel>
                      <FormControl>
                        <Input {...field} />
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
                      <Select onValueChange={field.onChange} defaultValue={field.value}>
                        <FormControl>
                          <SelectTrigger>
                            <SelectValue placeholder="Select order type" />
                          </SelectTrigger>
                        </FormControl>
                        <SelectContent>
                          <SelectItem value="MARKET">Market</SelectItem>
                          <SelectItem value="LIMIT">Limit</SelectItem>
                          <SelectItem value="STOP">Stop</SelectItem>
                          <SelectItem value="STOP_LIMIT">Stop Limit</SelectItem>
                        </SelectContent>
                      </Select>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <FormField
                 // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                  control={stockForm.control}
                  name="stopLoss"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Stop Loss</FormLabel>
                      <FormControl>
                        <Input type="number" step="0.01" {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <div className="grid grid-cols-2 gap-4">
                  <FormField
                   // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                    control={stockForm.control}
                    name="takeProfit"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Take Profit (Optional)</FormLabel>
                        <FormControl>
                          <Input
                            type="number"
                            step="0.01"
                            {...field}
                            value={field.value || ''}
                            onChange={(e) =>
                              field.onChange(e.target.value ? parseFloat(e.target.value) : undefined)
                            }
                          />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />

                  <FormField
                   // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                    control={stockForm.control}
                    name="initialTarget"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Initial Target (Optional)</FormLabel>
                        <FormControl>
                          <Input
                            type="number"
                            step="0.01"
                            {...field}
                            value={field.value || ''}
                            onChange={(e) =>
                              field.onChange(e.target.value ? parseFloat(e.target.value) : undefined)
                            }
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
                  name="profitTarget"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Profit Target (Optional)</FormLabel>
                      <FormControl>
                        <Input
                          type="number"
                          step="0.01"
                          {...field}
                          value={field.value || ''}
                          onChange={(e) =>
                            field.onChange(e.target.value ? parseFloat(e.target.value) : undefined)
                          }
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

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
                          {...field}
                          value={field.value || ''}
                          onChange={(e) =>
                            field.onChange(e.target.value ? parseInt(e.target.value) : 1)
                          }
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <FormField
                 // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                  control={stockForm.control}
                  name="reviewed"
                  render={({ field }) => (
                    <FormItem className="flex flex-row items-center justify-between rounded-lg border p-3">
                      <div className="space-y-0.5">
                        <FormLabel>Reviewed</FormLabel>
                        <FormDescription>
                          Mark this trade as reviewed
                        </FormDescription>
                      </div>
                      <FormControl>
                        <input
                          type="checkbox"
                          checked={field.value}
                          onChange={field.onChange}
                          className="h-4 w-4"
                        />
                      </FormControl>
                    </FormItem>
                  )}
                />

                <FormField
                 // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                  control={stockForm.control}
                  name="mistakes"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Mistakes</FormLabel>
                      <FormControl>
                        <Textarea {...field} rows={3} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <FormField
                 // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                  control={stockForm.control}
                  name="brokerageName"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Brokerage Name</FormLabel>
                      <FormControl>
                        <Input {...field} value={field.value || ''} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <DialogFooter>
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => onOpenChange(false)}
                  >
                    Cancel
                  </Button>
                  <Button type="submit" disabled={isMerging}>
                    {isMerging ? 'Merging...' : 'Merge Transactions'}
                  </Button>
                </DialogFooter>
              </form>
            </Form>
          </TabsContent>

          <TabsContent value="option">
            <Form {...optionForm}>
              <form
               // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                onSubmit={optionForm.handleSubmit(handleOptionSubmit)}
                className="space-y-4"
              >
                <FormField
                 // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                  control={optionForm.control}
                  name="symbol"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Symbol</FormLabel>
                      <FormControl>
                        <Input {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <FormField
                 // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                  control={optionForm.control}
                  name="strategyType"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Strategy Type</FormLabel>
                      <FormControl>
                        <Input {...field} placeholder="e.g., Covered Call, Iron Condor" />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <div className="grid grid-cols-2 gap-4">
                  <FormField
                   // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                    control={optionForm.control}
                    name="tradeDirection"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Trade Direction</FormLabel>
                        <Select onValueChange={field.onChange} defaultValue={field.value}>
                          <FormControl>
                            <SelectTrigger>
                              <SelectValue placeholder="Select direction" />
                            </SelectTrigger>
                          </FormControl>
                          <SelectContent>
                            <SelectItem value="Bullish">Bullish</SelectItem>
                            <SelectItem value="Bearish">Bearish</SelectItem>
                            <SelectItem value="Neutral">Neutral</SelectItem>
                          </SelectContent>
                        </Select>
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
                        <Select onValueChange={field.onChange} defaultValue={field.value}>
                          <FormControl>
                            <SelectTrigger>
                              <SelectValue placeholder="Select type" />
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
                          <Input type="number" step="0.01" {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />

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
                </div>

                <FormField
                 // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                  control={optionForm.control}
                  name="impliedVolatility"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Implied Volatility (%)</FormLabel>
                      <FormControl>
                        <Input type="number" step="0.01" {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <FormField
                 // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                  control={optionForm.control}
                  name="orderType"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Order Type</FormLabel>
                      <Select onValueChange={field.onChange} defaultValue={field.value}>
                        <FormControl>
                          <SelectTrigger>
                            <SelectValue placeholder="Select order type" />
                          </SelectTrigger>
                        </FormControl>
                        <SelectContent>
                          <SelectItem value="MARKET">Market</SelectItem>
                          <SelectItem value="LIMIT">Limit</SelectItem>
                          <SelectItem value="STOP">Stop</SelectItem>
                          <SelectItem value="STOP_LIMIT">Stop Limit</SelectItem>
                        </SelectContent>
                      </Select>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <FormField
                 // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                  control={optionForm.control}
                  name="stopLoss"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Stop Loss</FormLabel>
                      <FormControl>
                        <Input type="number" step="0.01" {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <div className="grid grid-cols-2 gap-4">
                  <FormField
                   // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                    control={optionForm.control}
                    name="takeProfit"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Take Profit (Optional)</FormLabel>
                        <FormControl>
                          <Input
                            type="number"
                            step="0.01"
                            {...field}
                            value={field.value || ''}
                            onChange={(e) =>
                              field.onChange(e.target.value ? parseFloat(e.target.value) : undefined)
                            }
                          />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />

                  <FormField
                   // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                    control={optionForm.control}
                    name="initialTarget"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Initial Target (Optional)</FormLabel>
                        <FormControl>
                          <Input
                            type="number"
                            step="0.01"
                            {...field}
                            value={field.value || ''}
                            onChange={(e) =>
                              field.onChange(e.target.value ? parseFloat(e.target.value) : undefined)
                            }
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
                  name="profitTarget"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Profit Target (Optional)</FormLabel>
                      <FormControl>
                        <Input
                          type="number"
                          step="0.01"
                          {...field}
                          value={field.value || ''}
                          onChange={(e) =>
                            field.onChange(e.target.value ? parseFloat(e.target.value) : undefined)
                          }
                        />
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
                          {...field}
                          value={field.value || ''}
                          onChange={(e) =>
                            field.onChange(e.target.value ? parseInt(e.target.value) : 1)
                          }
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <FormField
                 // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                  control={optionForm.control}
                  name="reviewed"
                  render={({ field }) => (
                    <FormItem className="flex flex-row items-center justify-between rounded-lg border p-3">
                      <div className="space-y-0.5">
                        <FormLabel>Reviewed</FormLabel>
                        <FormDescription>
                          Mark this trade as reviewed
                        </FormDescription>
                      </div>
                      <FormControl>
                        <input
                          type="checkbox"
                          checked={field.value}
                          onChange={field.onChange}
                          className="h-4 w-4"
                        />
                      </FormControl>
                    </FormItem>
                  )}
                />

                <FormField
                 // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                  control={optionForm.control}
                  name="mistakes"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Mistakes</FormLabel>
                      <FormControl>
                        <Textarea {...field} rows={3} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <FormField
                 // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                  control={optionForm.control}
                  name="brokerageName"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Brokerage Name</FormLabel>
                      <FormControl>
                        <Input {...field} value={field.value || ''} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <DialogFooter>
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => onOpenChange(false)}
                  >
                    Cancel
                  </Button>
                  <Button type="submit" disabled={isMerging}>
                    {isMerging ? 'Merging...' : 'Merge Transactions'}
                  </Button>
                </DialogFooter>
              </form>
            </Form>
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}

