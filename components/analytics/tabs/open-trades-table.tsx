'use client';

import { useMemo } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Skeleton } from '@/components/ui/skeleton';
import { Badge } from '@/components/ui/badge';
import { useStocks } from '@/lib/hooks/use-stocks';
import { useOptions } from '@/lib/hooks/use-options';
import { useQuotes } from '@/lib/hooks/use-market-data-service';
import { cn } from '@/lib/utils';
import Image from 'next/image';

interface OpenTrade {
  id: string;
  symbol: string;
  type: 'stock' | 'option';
  entryPrice: number;
  currentPrice: number | null;
  quantity: number;
  quantityLabel: string; // "shares" or "contracts"
  pnl: number | null;
  pnlPercent: number | null;
  tradeType?: 'BUY' | 'SELL'; // For stocks
  optionType?: 'Call' | 'Put'; // For options
  logo?: string | null;
}

export function OpenTradesTable() {
  const { data: stocks = [], isLoading: stocksLoading } = useStocks();
  const { data: options = [], isLoading: optionsLoading } = useOptions();

  // Filter open trades
  const openTrades = useMemo(() => {
    const trades: OpenTrade[] = [];

    // Filter open stock trades (exitPrice is null or exitDate is null)
    const openStocks = stocks.filter(
      (stock) => !stock.exitPrice || !stock.exitDate
    );

    openStocks.forEach((stock) => {
      trades.push({
        id: `stock-${stock.id}`,
        symbol: stock.symbol,
        type: 'stock',
        entryPrice: parseFloat(stock.entryPrice),
        currentPrice: null, // Will be populated from quotes
        quantity: parseFloat(stock.numberShares),
        quantityLabel: 'shares',
        pnl: null,
        pnlPercent: null,
        tradeType: stock.tradeType,
      });
    });

    // Filter open option trades (status === 'open')
    const openOptions = options.filter((option) => option.status === 'open');

    openOptions.forEach((option) => {
      trades.push({
        id: `option-${option.id}`,
        symbol: option.symbol,
        type: 'option',
        entryPrice: parseFloat(option.entryPrice),
        currentPrice: null, // Will be populated from quotes
        // @ts-ignore
        quantity: option.quantity ? parseFloat(option.quantity) : 1, // Use quantity (contracts) instead of numberOfContracts
        quantityLabel: 'contracts',
        pnl: null,
        pnlPercent: null,
        optionType: option.optionType,
      });
    });

    return trades;
  }, [stocks, options]);

  // Get unique symbols from open trades
  const symbols = useMemo(() => {
    return Array.from(new Set(openTrades.map((trade) => trade.symbol)));
  }, [openTrades]);

  // Fetch quotes for all symbols (this provides current price and logo)
  const { quotes, isLoading: quotesLoading } = useQuotes(
    { symbols },
    symbols.length > 0
  );

  // Create a map of symbol to quote data
  const quoteMap = useMemo(() => {
    const map = new Map<string, { price: number; logo?: string | null }>();
    quotes.forEach((quote) => {
      if (quote.price !== undefined && quote.price !== null) {
        map.set(quote.symbol.toUpperCase(), {
          price: typeof quote.price === 'string' ? parseFloat(quote.price) : quote.price,
          logo: quote.logo || null,
        });
      }
    });
    return map;
  }, [quotes]);

  // Calculate P&L for each trade
  const tradesWithPnL = useMemo(() => {
    return openTrades.map((trade) => {
      const quote = quoteMap.get(trade.symbol.toUpperCase());
      const currentPrice = quote?.price ?? null;
      let pnl: number | null = null;
      let pnlPercent: number | null = null;

      if (currentPrice !== null) {
        if (trade.type === 'stock') {
          // For stocks: BUY = (current - entry) * shares, SELL = (entry - current) * shares
          if (trade.tradeType === 'BUY') {
            pnl = (currentPrice - trade.entryPrice) * trade.quantity;
          } else if (trade.tradeType === 'SELL') {
            pnl = (trade.entryPrice - currentPrice) * trade.quantity;
          }
        } else if (trade.type === 'option') {
          // For options: (current - entry) * contracts * 100
          pnl = (currentPrice - trade.entryPrice) * trade.quantity * 100;
        }

        // Calculate percentage
        if (trade.entryPrice > 0) {
          pnlPercent = ((currentPrice - trade.entryPrice) / trade.entryPrice) * 100;
        }
      }

      return {
        ...trade,
        currentPrice,
        pnl,
        pnlPercent,
        logo: quote?.logo ?? null,
      };
    });
  }, [openTrades, quoteMap]);

  const isLoading = stocksLoading || optionsLoading || quotesLoading;

  const formatCurrency = (value: number): string => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(value);
  };

  const formatNumber = (value: number): string => {
    return new Intl.NumberFormat('en-US', {
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(value);
  };

  const getPnLColor = (pnl: number | null): string => {
    if (pnl === null) return 'text-muted-foreground';
    if (pnl > 0) return 'text-green-600 dark:text-green-400';
    if (pnl < 0) return 'text-red-600 dark:text-red-400';
    return 'text-muted-foreground';
  };

  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Open Trades</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            {Array.from({ length: 5 }).map((_, i) => (
              <Skeleton key={i} className="h-12 w-full" />
            ))}
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Open Trades</CardTitle>
        <p className="text-sm text-muted-foreground">
          {tradesWithPnL.length} {tradesWithPnL.length === 1 ? 'trade' : 'trades'} currently open
        </p>
      </CardHeader>
      <CardContent>
        {tradesWithPnL.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            No open trades
          </div>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-12"></TableHead>
                <TableHead>Symbol</TableHead>
                <TableHead>Type</TableHead>
                <TableHead className="text-right">Entry Price</TableHead>
                <TableHead className="text-right">Current Price</TableHead>
                <TableHead className="text-right">Quantity</TableHead>
                <TableHead className="text-right">P&L</TableHead>
                <TableHead className="text-right">P&L %</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {tradesWithPnL.map((trade) => (
                <TableRow key={trade.id}>
                  <TableCell>
                    {trade.logo ? (
                      <Image
                        src={trade.logo}
                        alt={trade.symbol}
                        width={32}
                        height={32}
                        className="rounded-full"
                      />
                    ) : (
                      <div className="w-8 h-8 rounded-full bg-muted flex items-center justify-center text-xs font-medium">
                        {trade.symbol.slice(0, 2).toUpperCase()}
                      </div>
                    )}
                  </TableCell>
                  <TableCell className="font-medium">{trade.symbol}</TableCell>
                  <TableCell>
                    {trade.type === 'stock' ? (
                      <Badge variant="outline">{trade.tradeType}</Badge>
                    ) : (
                      <Badge variant="outline">{trade.optionType}</Badge>
                    )}
                  </TableCell>
                  <TableCell className="text-right">
                    {formatCurrency(trade.entryPrice)}
                  </TableCell>
                  <TableCell className="text-right">
                    {trade.currentPrice !== null ? (
                      formatCurrency(trade.currentPrice)
                    ) : (
                      <span className="text-muted-foreground">—</span>
                    )}
                  </TableCell>
                  <TableCell className="text-right">
                    {formatNumber(trade.quantity)} {trade.quantityLabel}
                  </TableCell>
                  <TableCell className={cn('text-right font-medium', getPnLColor(trade.pnl))}>
                    {trade.pnl !== null ? (
                      <>
                        {trade.pnl >= 0 ? '+' : ''}
                        {formatCurrency(trade.pnl)}
                      </>
                    ) : (
                      <span className="text-muted-foreground">—</span>
                    )}
                  </TableCell>
                  <TableCell className={cn('text-right', getPnLColor(trade.pnl))}>
                    {trade.pnlPercent !== null ? (
                      <>
                        {trade.pnlPercent >= 0 ? '+' : ''}
                        {trade.pnlPercent.toFixed(2)}%
                      </>
                    ) : (
                      <span className="text-muted-foreground">—</span>
                    )}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </CardContent>
    </Card>
  );
}
