'use client';

import { useMemo, useState } from 'react';
import Image from 'next/image';
import { useStocks } from '@/lib/hooks/use-stocks';
import { useOptions } from '@/lib/hooks/use-options';
import { useQuote, useMarketSubscription } from '@/lib/hooks/use-market-data-service';
import { useLogo } from '@/lib/hooks/use-market-data-service';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  Pagination,
  PaginationContent,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from '@/components/ui/pagination';
import type { Stock } from '@/lib/types/stocks';
import type { OptionTrade } from '@/lib/types/options';
import { format } from 'date-fns';

type TradeItem = {
  id: number;
  type: 'stock' | 'option';
  symbol: string;
  entryPrice: number;
  entryDate: string;
  exitPrice?: number | null;
  exitDate?: string | null;
  quantity: number;
  currentPrice?: number | null;
  companyName?: string | null;
  logo?: string | null;
  // Option-specific fields
  optionType?: 'Call' | 'Put';
  strikePrice?: number;
  expirationDate?: string;
  // Stock-specific fields
  numberShares?: number;
};

function TradeRow({ trade }: { trade: TradeItem }) {
  const { quote } = useQuote(trade.symbol, true);
  const { logo } = useLogo(trade.symbol, true);
  const [logoError, setLogoError] = useState(false);

  // Use real-time price from WebSocket if available, otherwise use cached quote
  const currentPrice = quote?.price ? parseFloat(quote.price) : trade.currentPrice ?? null;
  const companyName = quote?.name ?? trade.companyName ?? trade.symbol;

  // Calculate P&L
  const calculatePnL = () => {
    if (trade.exitPrice) {
      // Closed trade - use exit price
      const entry = trade.entryPrice;
      const exit = parseFloat(trade.exitPrice.toString());
      const pnl = (exit - entry) * trade.quantity;
      const pnlPercent = ((exit - entry) / entry) * 100;
      return { pnl, pnlPercent, isPositive: pnl >= 0 };
    } else if (currentPrice) {
      // Open trade - use current price
      const entry = trade.entryPrice;
      const current = currentPrice;
      const pnl = (current - entry) * trade.quantity;
      const pnlPercent = ((current - entry) / entry) * 100;
      return { pnl, pnlPercent, isPositive: pnl >= 0 };
    }
    return null;
  };

  const pnlData = calculatePnL();

  return (
    <TableRow>
      <TableCell className="flex items-center gap-2">
        {logo && !logoError && (
          <Image
            src={logo}
            alt={companyName}
            width={32}
            height={32}
            className="rounded-full object-cover"
            onError={() => setLogoError(true)}
            unoptimized
          />
        )}
        <div>
          <div className="font-medium">{trade.symbol}</div>
          <div className="text-xs text-muted-foreground">{companyName}</div>
        </div>
      </TableCell>
      <TableCell>
        <span className="inline-flex items-center rounded-full px-2 py-1 text-xs font-medium bg-muted">
          {trade.type === 'stock' ? 'Stock' : 'Option'}
        </span>
        {trade.optionType && (
          <span className="ml-1 text-xs text-muted-foreground">
            ({trade.optionType})
          </span>
        )}
      </TableCell>
      <TableCell className="text-right">
        {trade.quantity.toLocaleString(undefined, {
          minimumFractionDigits: 0,
          maximumFractionDigits: 2,
        })}
      </TableCell>
      <TableCell className="text-right">
        ${trade.entryPrice.toFixed(2)}
      </TableCell>
      <TableCell className="text-right">
        {currentPrice ? (
          <span className="font-medium">${currentPrice.toFixed(2)}</span>
        ) : (
          <span className="text-muted-foreground">—</span>
        )}
      </TableCell>
      {trade.exitPrice ? (
        <TableCell className="text-right">
          ${parseFloat(trade.exitPrice.toString()).toFixed(2)}
        </TableCell>
      ) : (
        <TableCell className="text-right text-muted-foreground">—</TableCell>
      )}
      {pnlData ? (
        <TableCell
          className={`text-right font-medium ${
            pnlData.isPositive ? 'text-green-600' : 'text-red-600'
          }`}
        >
          <div>
            {pnlData.isPositive ? '+' : ''}
            ${pnlData.pnl.toFixed(2)}
          </div>
          <div className="text-xs">
            ({pnlData.isPositive ? '+' : ''}
            {pnlData.pnlPercent.toFixed(2)}%)
          </div>
        </TableCell>
      ) : (
        <TableCell className="text-right text-muted-foreground">—</TableCell>
      )}
      <TableCell className="text-muted-foreground">
        {format(new Date(trade.entryDate), 'MMM d, yyyy')}
      </TableCell>
      {trade.exitDate && (
        <TableCell className="text-muted-foreground">
          {format(new Date(trade.exitDate), 'MMM d, yyyy')}
        </TableCell>
      )}
    </TableRow>
  );
}

export function Trades() {
  const [openPage, setOpenPage] = useState(1);
  const [closedPage, setClosedPage] = useState(1);
  const itemsPerPage = 5;

  const { data: openStocks, isLoading: openStocksLoading } = useStocks({ openOnly: true });
  const { data: closedStocks, isLoading: closedStocksLoading } = useStocks({ openOnly: false });
  const { data: openOptions, isLoading: openOptionsLoading } = useOptions({ openOnly: true });
  const { data: closedOptions, isLoading: closedOptionsLoading } = useOptions({ openOnly: false });

  // Combine stocks and options into unified trade items
  const openTrades = useMemo(() => {
    const trades: TradeItem[] = [];

    // Add open stocks
    if (openStocks) {
      openStocks.forEach((stock: Stock) => {
        trades.push({
          id: stock.id,
          type: 'stock',
          symbol: stock.symbol,
          entryPrice: parseFloat(stock.entryPrice),
          entryDate: stock.entryDate,
          exitPrice: stock.exitPrice ? parseFloat(stock.exitPrice) : null,
          exitDate: stock.exitDate,
          quantity: parseFloat(stock.numberShares),
          numberShares: parseFloat(stock.numberShares),
        });
      });
    }

    // Add open options
    if (openOptions) {
      openOptions.forEach((option: OptionTrade) => {
        trades.push({
          id: option.id,
          type: 'option',
          symbol: option.symbol,
          entryPrice: parseFloat(option.entryPrice),
          entryDate: option.entryDate,
          exitPrice: option.exitPrice ? parseFloat(option.exitPrice) : null,
          exitDate: option.exitDate,
          // @ts-ignore
          quantity: option.quantity ? parseFloat(option.quantity) : 1, // Use quantity (contracts) instead of numberOfContracts
          optionType: option.optionType,
          strikePrice: parseFloat(option.strikePrice),
          expirationDate: option.expirationDate,
        });
      });
    }

    // Sort by entry date (newest first)
    return trades.sort(
      (a, b) => new Date(b.entryDate).getTime() - new Date(a.entryDate).getTime()
    );
  }, [openStocks, openOptions]);

  const closedTrades = useMemo(() => {
    const trades: TradeItem[] = [];

    // Add closed stocks
    if (closedStocks) {
      closedStocks.forEach((stock: Stock) => {
        trades.push({
          id: stock.id,
          type: 'stock',
          symbol: stock.symbol,
          entryPrice: parseFloat(stock.entryPrice),
          entryDate: stock.entryDate,
          exitPrice: stock.exitPrice ? parseFloat(stock.exitPrice) : null,
          exitDate: stock.exitDate,
          quantity: parseFloat(stock.numberShares),
          numberShares: parseFloat(stock.numberShares),
        });
      });
    }

    // Add closed options
    if (closedOptions) {
      closedOptions.forEach((option: OptionTrade) => {
        trades.push({
          id: option.id,
          type: 'option',
          symbol: option.symbol,
          entryPrice: parseFloat(option.entryPrice),
          entryDate: option.entryDate,
          exitPrice: option.exitPrice ? parseFloat(option.exitPrice) : null,
          exitDate: option.exitDate,
          // @ts-ignore
          quantity: option.quantity ? parseFloat(option.quantity) : 1, // Use quantity (contracts) instead of numberOfContracts
          optionType: option.optionType,
          strikePrice: parseFloat(option.strikePrice),
          expirationDate: option.expirationDate,
        });
      });
    }

    // Sort by exit date (newest first)
    return trades.sort(
      (a, b) =>
        (b.exitDate ? new Date(b.exitDate).getTime() : 0) -
        (a.exitDate ? new Date(a.exitDate).getTime() : 0)
    );
  }, [closedStocks, closedOptions]);

  // Get all unique symbols for WebSocket subscription (only for open trades)
  const openSymbols = useMemo(() => {
    const symbols = new Set<string>();
    openTrades.forEach((trade) => symbols.add(trade.symbol));
    return Array.from(symbols);
  }, [openTrades]);

  // Subscribe to WebSocket updates for all open trade symbols
  useMarketSubscription(openSymbols, openSymbols.length > 0);

  // Paginate trades
  const paginatedOpenTrades = useMemo(() => {
    const start = (openPage - 1) * itemsPerPage;
    const end = start + itemsPerPage;
    return openTrades.slice(start, end);
  }, [openTrades, openPage]);

  const paginatedClosedTrades = useMemo(() => {
    const start = (closedPage - 1) * itemsPerPage;
    const end = start + itemsPerPage;
    return closedTrades.slice(start, end);
  }, [closedTrades, closedPage]);

  const openTotalPages = Math.ceil(openTrades.length / itemsPerPage);
  const closedTotalPages = Math.ceil(closedTrades.length / itemsPerPage);

  const isLoading =
    openStocksLoading || closedStocksLoading || openOptionsLoading || closedOptionsLoading;

  return (
    <Card className="h-full flex flex-col">
      <CardHeader>
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Trades</h2>
          <p className="text-muted-foreground">
            View and manage your stock and option trades
          </p>
        </div>
      </CardHeader>
      <CardContent className="flex-1 min-h-[400px]">
        <Tabs 
          defaultValue="open" 
          className="w-full h-full"
          onValueChange={(value) => {
            if (value === 'open') {
              setOpenPage(1);
            } else {
              setClosedPage(1);
            }
          }}
        >
        <TabsList>
          <TabsTrigger value="open">
            Open ({openTrades.length})
          </TabsTrigger>
          <TabsTrigger value="closed">
            Closed ({closedTrades.length})
          </TabsTrigger>
        </TabsList>

        <TabsContent value="open" className="space-y-4">
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <div className="text-muted-foreground">Loading trades...</div>
            </div>
          ) : openTrades.length === 0 ? (
            <div className="flex items-center justify-center py-8">
              <div className="text-muted-foreground">No open trades</div>
            </div>
          ) : (
            <>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Symbol</TableHead>
                    <TableHead>Type</TableHead>
                    <TableHead className="text-right">Quantity</TableHead>
                    <TableHead className="text-right">Entry Price</TableHead>
                    <TableHead className="text-right">Current Price</TableHead>
                    <TableHead className="text-right">P&L</TableHead>
                    <TableHead>Entry Date</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {paginatedOpenTrades.map((trade) => (
                    <TradeRow key={`${trade.type}-${trade.id}`} trade={trade} />
                  ))}
                </TableBody>
              </Table>
              {openTotalPages > 1 && (
                <Pagination>
                  <PaginationContent>
                    <PaginationItem>
                      <PaginationPrevious
                        href="#"
                        onClick={(e) => {
                          e.preventDefault();
                          setOpenPage((prev) => Math.max(1, prev - 1));
                        }}
                        className={openPage === 1 ? 'pointer-events-none opacity-50' : ''}
                      />
                    </PaginationItem>
                    {Array.from({ length: openTotalPages }).map((_, idx) => {
                      const pageNum = idx + 1;
                      const maxVisiblePages = 5;
                      const startPage = Math.max(1, openPage - Math.floor(maxVisiblePages / 2));
                      const endPage = Math.min(openTotalPages, startPage + maxVisiblePages - 1);
                      
                      if (pageNum < startPage || pageNum > endPage) {
                        if (pageNum === startPage - 1 || pageNum === endPage + 1) {
                          return (
                            <PaginationItem key={`ellipsis-${pageNum}`}>
                              <span className="flex size-9 items-center justify-center text-muted-foreground">
                                ...
                              </span>
                            </PaginationItem>
                          );
                        }
                        return null;
                      }
                      
                      return (
                        <PaginationItem key={pageNum}>
                          <PaginationLink
                            href="#"
                            isActive={pageNum === openPage}
                            onClick={(e) => {
                              e.preventDefault();
                              setOpenPage(pageNum);
                            }}
                          >
                            {pageNum}
                          </PaginationLink>
                        </PaginationItem>
                      );
                    })}
                    <PaginationItem>
                      <PaginationNext
                        href="#"
                        onClick={(e) => {
                          e.preventDefault();
                          setOpenPage((prev) => Math.min(openTotalPages, prev + 1));
                        }}
                        className={openPage === openTotalPages ? 'pointer-events-none opacity-50' : ''}
                      />
                    </PaginationItem>
                  </PaginationContent>
                </Pagination>
              )}
            </>
          )}
        </TabsContent>

        <TabsContent value="closed" className="space-y-4">
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <div className="text-muted-foreground">Loading trades...</div>
            </div>
          ) : closedTrades.length === 0 ? (
            <div className="flex items-center justify-center py-8">
              <div className="text-muted-foreground">No closed trades</div>
            </div>
          ) : (
            <>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Symbol</TableHead>
                    <TableHead>Type</TableHead>
                    <TableHead className="text-right">Quantity</TableHead>
                    <TableHead className="text-right">Entry Price</TableHead>
                    <TableHead className="text-right">Exit Price</TableHead>
                    <TableHead className="text-right">P&L</TableHead>
                    <TableHead>Entry Date</TableHead>
                    <TableHead>Exit Date</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {paginatedClosedTrades.map((trade) => (
                    <TradeRow key={`${trade.type}-${trade.id}`} trade={trade} />
                  ))}
                </TableBody>
              </Table>
              {closedTotalPages > 1 && (
                <Pagination>
                  <PaginationContent>
                    <PaginationItem>
                      <PaginationPrevious
                        href="#"
                        onClick={(e) => {
                          e.preventDefault();
                          setClosedPage((prev) => Math.max(1, prev - 1));
                        }}
                        className={closedPage === 1 ? 'pointer-events-none opacity-50' : ''}
                      />
                    </PaginationItem>
                    {Array.from({ length: closedTotalPages }).map((_, idx) => {
                      const pageNum = idx + 1;
                      const maxVisiblePages = 5;
                      const startPage = Math.max(1, closedPage - Math.floor(maxVisiblePages / 2));
                      const endPage = Math.min(closedTotalPages, startPage + maxVisiblePages - 1);
                      
                      if (pageNum < startPage || pageNum > endPage) {
                        if (pageNum === startPage - 1 || pageNum === endPage + 1) {
                          return (
                            <PaginationItem key={`ellipsis-${pageNum}`}>
                              <span className="flex size-9 items-center justify-center text-muted-foreground">
                                ...
                              </span>
                            </PaginationItem>
                          );
                        }
                        return null;
                      }
                      
                      return (
                        <PaginationItem key={pageNum}>
                          <PaginationLink
                            href="#"
                            isActive={pageNum === closedPage}
                            onClick={(e) => {
                              e.preventDefault();
                              setClosedPage(pageNum);
                            }}
                          >
                            {pageNum}
                          </PaginationLink>
                        </PaginationItem>
                      );
                    })}
                    <PaginationItem>
                      <PaginationNext
                        href="#"
                        onClick={(e) => {
                          e.preventDefault();
                          setClosedPage((prev) => Math.min(closedTotalPages, prev + 1));
                        }}
                        className={closedPage === closedTotalPages ? 'pointer-events-none opacity-50' : ''}
                      />
                    </PaginationItem>
                  </PaginationContent>
                </Pagination>
              )}
            </>
          )}
        </TabsContent>
        </Tabs>
      </CardContent>
    </Card>
  );
}