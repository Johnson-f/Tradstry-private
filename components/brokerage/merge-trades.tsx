'use client';

import React, { useMemo, useState } from 'react';
import { useBrokerageTransactions } from '@/lib/hooks/use-brokerage';
import type { BrokerageTransaction } from '@/lib/types/brokerage';
import { isOptionTransaction } from '@/lib/utils/brokerage';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Checkbox } from '@/components/ui/checkbox';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { MergeTradeForm } from './merge-trade-form';
import { format } from 'date-fns';

export function MergeTrades() {
  const { transactions, loading, error, refetch } = useBrokerageTransactions({
     // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
    exclude_transformed: true,
  });
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [searchSymbol, setSearchSymbol] = useState('');
  const [isFormOpen, setIsFormOpen] = useState(false);

  // Apply search filter
  const filteredTransactions = useMemo(() => {
    if (!searchSymbol) return transactions;
    return transactions.filter((txn) => {
      if (txn.symbol) {
        return txn.symbol.toLowerCase().includes(searchSymbol.toLowerCase());
      }
      return false;
    });
  }, [transactions, searchSymbol]);

  // Group transactions by symbol
  const groupedTransactions = useMemo(() => {
    const groups = new Map<string, BrokerageTransaction[]>();
    filteredTransactions.forEach((txn) => {
      const symbol = txn.symbol || 'Unknown';
      if (!groups.has(symbol)) {
        groups.set(symbol, []);
      }
      groups.get(symbol)!.push(txn);
    });
    return Array.from(groups.entries()).sort(([a], [b]) => a.localeCompare(b));
  }, [filteredTransactions]);

  const handleSelectTransaction = (id: string, checked: boolean) => {
    const newSelected = new Set(selectedIds);
    if (checked) {
      newSelected.add(id);
    } else {
      newSelected.delete(id);
    }
    setSelectedIds(newSelected);
  };

  const handleMerge = () => {
    if (selectedIds.size === 0) {
      return;
    }
    setIsFormOpen(true);
  };

  const selectedTransactions = useMemo(() => {
    return filteredTransactions.filter((t) => selectedIds.has(t.id));
  }, [filteredTransactions, selectedIds]);

  const handleFormClose = () => {
    setIsFormOpen(false);
    setSelectedIds(new Set());
    void refetch();
  };

  if (loading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Merge Trades</CardTitle>
          <CardDescription>Loading transactions...</CardDescription>
        </CardHeader>
      </Card>
    );
  }

  if (error) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Merge Trades</CardTitle>
          <CardDescription>Error loading transactions: {error.message}</CardDescription>
        </CardHeader>
      </Card>
    );
  }

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>Merge Trades</CardTitle>
          <CardDescription>
            Select transactions to merge into a stock or option trade
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center gap-4">
            <Input
              placeholder="Search by symbol..."
              value={searchSymbol}
              onChange={(e) => setSearchSymbol(e.target.value)}
              className="max-w-sm"
            />
            <div className="flex-1" />
            <Button
              onClick={handleMerge}
              disabled={selectedIds.size === 0}
            >
              Merge Selected ({selectedIds.size})
            </Button>
          </div>

          {filteredTransactions.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              {searchSymbol
                ? 'No untransformed transactions found matching your search'
                : 'No untransformed transactions available'}
            </div>
          ) : (
            <div className="space-y-6">
              {groupedTransactions.map(([symbol, txns]) => (
                <div key={symbol} className="space-y-2">
                  <div className="flex items-center gap-2">
                    <h3 className="font-semibold">{symbol}</h3>
                    <Badge variant="secondary">{txns.length} transaction(s)</Badge>
                    {txns.some((t) => isOptionTransaction(t)) && (
                      <Badge variant="outline">Options</Badge>
                    )}
                  </div>
                  <div className="border rounded-lg">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead className="w-12">
                            <Checkbox
                              checked={
                                txns.length > 0 &&
                                txns.every((t) => selectedIds.has(t.id))
                              }
                              onCheckedChange={(checked) => {
                                txns.forEach((t) => {
                                  handleSelectTransaction(t.id, checked as boolean);
                                });
                              }}
                            />
                          </TableHead>
                          <TableHead>Type</TableHead>
                          <TableHead>Quantity</TableHead>
                          <TableHead>Price</TableHead>
                          <TableHead>Fees</TableHead>
                          <TableHead>Trade Date</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {txns.map((txn) => (
                          <TableRow key={txn.id}>
                            <TableCell>
                              <Checkbox
                                checked={selectedIds.has(txn.id)}
                                onCheckedChange={(checked) =>
                                  handleSelectTransaction(txn.id, checked as boolean)
                                }
                              />
                            </TableCell>
                            <TableCell>
                              <Badge
                                variant={
                                  txn.transaction_type === 'BUY'
                                    ? 'default'
                                    : 'destructive'
                                }
                              >
                                {txn.transaction_type || 'N/A'}
                              </Badge>
                            </TableCell>
                            <TableCell>{txn.quantity?.toFixed(2) || 'N/A'}</TableCell>
                            <TableCell>${txn.price?.toFixed(2) || 'N/A'}</TableCell>
                            <TableCell>${txn.fees?.toFixed(2) || '0.00'}</TableCell>
                            <TableCell>
                              {txn.trade_date
                                ? format(new Date(txn.trade_date), 'MMM dd, yyyy')
                                : 'N/A'}
                            </TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <MergeTradeForm
        open={isFormOpen}
        onOpenChange={setIsFormOpen}
        transactions={selectedTransactions}
        onSuccess={handleFormClose}
      />
    </>
  );
}

