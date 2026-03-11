import type { BrokerageTransaction } from '@/lib/types/brokerage';

/**
 * Check if a transaction is an option trade by examining raw_data
 */
export function isOptionTransaction(transaction: BrokerageTransaction): boolean {
  if (!transaction.raw_data) {
    return false;
  }

  try {
    const rawData = typeof transaction.raw_data === 'string' 
      ? JSON.parse(transaction.raw_data)
      : transaction.raw_data;

    // Check for option-specific fields
    const hasOptionSymbol = rawData.option_symbol && 
      typeof rawData.option_symbol === 'string' && 
      rawData.option_symbol.trim() !== '';
    
    const hasOptionType = rawData.option_type && 
      typeof rawData.option_type === 'string' && 
      rawData.option_type.trim() !== '';

    return hasOptionSymbol || hasOptionType;
  } catch (error) {
    console.error('Failed to parse raw_data for transaction:', error);
    return false;
  }
}

/**
 * Calculate weighted average price from multiple transactions
 */
export function calculateWeightedAverage(
  transactions: BrokerageTransaction[]
): { price: number; quantity: number; fees: number } {
  let totalValue = 0;
  let totalQuantity = 0;
  let totalFees = 0;

  for (const txn of transactions) {
    const qty = txn.quantity || 0;
    const price = txn.price || 0;
    const fees = txn.fees || 0;

    totalValue += price * qty;
    totalQuantity += qty;
    totalFees += fees;
  }

  const avgPrice = totalQuantity > 0 ? totalValue / totalQuantity : 0;

  return {
    price: avgPrice,
    quantity: totalQuantity,
    fees: totalFees,
  };
}

