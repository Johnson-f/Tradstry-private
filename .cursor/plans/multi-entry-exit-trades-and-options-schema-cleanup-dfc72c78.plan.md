<!-- dfc72c78-5b4e-4873-8abf-5e2de0088743 1d415925-e77e-4f88-95aa-13648ecd0a00 -->
# Multi-Entry/Exit Trades with Parent-Child Linking & Hybrid Positions

## Overview

Enable users to record multiple buy and sell transactions using parent-child trade linking, update options table schema (remove unnecessary columns, rename total_premium to premium), add performance indexes, and implement hybrid position tracking for brokerage integration.

## Implementation Plan

### Phase 1: Options Table Schema Cleanup

**File: `backend/src/turso/schema.rs`**

- Remove columns from options table:
- `strategy_type`
- `trade_direction` 
- `number_of_contracts`
- `commissions`
- `implied_volatility`
- Rename `total_premium` to `premium`
- Update schema version to 0.0.29
- Add migration logic to handle column removal (backup, recreate, restore data)
- Update `get_expected_schema()` function to reflect new options schema

**File: `backend/src/models/options/option_trade.rs`**

- Remove fields from `OptionTrade` struct: `strategy_type`, `trade_direction`, `number_of_contracts`, `commissions`, `implied_volatility`
- Rename `total_premium` to `premium`
- Update `CreateOptionRequest` and `UpdateOptionRequest` DTOs
- Update all SQL queries to remove references to deleted columns
- Update `from_row()` method to match new column order
- Update calculation methods to work without removed columns
- Update `data_formatter.rs` to use new schema

### Phase 2: Add Indexes

**File: `backend/src/turso/schema.rs`**

**Stocks table indexes (new):**

- `entry_price` 
- `exit_price`
- `brokerage_name`

**Options table indexes (new):**

- `strike_price`
- `entry_price`
- `exit_price`
- `brokerage_name`

(Keep existing indexes: symbol, trade_type, entry_date, exit_date, option_type, status, expiration_date)

### Phase 3: Parent-Child Trade Linking

**File: `backend/src/turso/schema.rs`**

- Add `trade_group_id` TEXT column to both `stocks` and `options` tables (UUID for grouping related trades)
- Add `parent_trade_id` INTEGER column (nullable, references id of parent trade)
- Add `quantity` DECIMAL(15,8) column to both tables (for stocks: shares, for options: contracts)
- Add `transaction_sequence` INTEGER column (order within trade group)
- Add indexes on `trade_group_id`, `parent_trade_id`
- Update schema version

**File: `backend/src/models/stock/stocks.rs`**

- Add `trade_group_id`, `parent_trade_id`, `quantity`, `transaction_sequence` fields to `Stock` struct
- Update `CreateStockRequest` to include these optional fields
- Add method `create_linked_trade()` to create child trades linked to parent
- Add method `get_trade_group(group_id)` to retrieve all trades in a group
- Add method `get_child_trades(parent_id)` to get all child trades
- Add method `calculate_group_pnl(group_id)` for aggregate P&L
- Update `find_all()` to support filtering by `trade_group_id` or `parent_trade_id`
- Update P&L calculations to optionally aggregate by trade group

**File: `backend/src/models/options/option_trade.rs`**

- Add `trade_group_id`, `parent_trade_id`, `quantity`, `transaction_sequence` fields to `OptionTrade` struct
- Update `CreateOptionRequest` to include these optional fields
- Add method `create_linked_trade()` to create child trades linked to parent
- Add method `get_trade_group(group_id)` to retrieve all trades in a group
- Add method `get_child_trades(parent_id)` to get all child trades
- Add method `calculate_group_pnl(group_id)` for aggregate P&L
- Update `find_all()` to support filtering by `trade_group_id` or `parent_trade_id`
- Update P&L calculations to optionally aggregate by trade group

### Phase 4: Hybrid Position Tracking (Brokerage Integration)

**File: `backend/src/turso/schema.rs`**

- Create new `stock_positions` table:
- `id` TEXT PRIMARY KEY
- `symbol` TEXT NOT NULL
- `total_quantity` DECIMAL(15,8) NOT NULL
- `average_entry_price` DECIMAL(15,8) NOT NULL
- `current_price` DECIMAL(15,8)
- `unrealized_pnl` DECIMAL(15,8)
- `realized_pnl` DECIMAL(15,8) DEFAULT 0
- `brokerage_name` TEXT
- `created_at` TIMESTAMP
- `updated_at` TIMESTAMP
- Create new `option_positions` table (add `strike_price`, `expiration_date`, `option_type`)
- Create new `position_transactions` audit log table:
- `id` TEXT PRIMARY KEY
- `position_id` TEXT NOT NULL
- `position_type` TEXT CHECK (position_type IN ('stock', 'option'))
- `trade_id` INTEGER (references stocks.id or options.id)
- `transaction_type` TEXT CHECK (transaction_type IN ('entry', 'exit', 'adjustment'))
- `quantity` DECIMAL(15,8) NOT NULL
- `price` DECIMAL(15,8) NOT NULL
- `transaction_date` TIMESTAMP NOT NULL
- `created_at` TIMESTAMP
- Add indexes on position tables and transaction log

**File: `backend/src/models/stock/stocks.rs`**

- Add method `sync_to_position()` to update/create position record when trade is created
- Add method `get_position(symbol)` to retrieve position summary
- Add method `get_position_transactions(symbol)` to get transaction history

**File: `backend/src/models/options/option_trade.rs`**

- Add method `sync_to_position()` to update/create position record when trade is created
- Add method `get_position(symbol, strike_price, expiration_date)` to retrieve position summary
- Add method `get_position_transactions(symbol, strike_price, expiration_date)` to get transaction history

**File: `backend/src/service/position_service.rs` (NEW)**

- Create service module for position management:
- `calculate_weighted_average_entry()` - for multiple entries
- `calculate_realized_pnl()` - FIFO or average cost basis
- `update_position_quantity()` - after entry/exit
- `reconcile_position()` - verify position matches transaction history

### Phase 5: Brokerage Merge Logic Update (Hybrid Approach)

**File: `backend/src/routes/brokerage.rs`**

- Update `merge_transactions()` function to:
- Group transactions by symbol (and for options: strike_price + expiration_date + option_type)
- Create or update position records with aggregated quantities and weighted average prices
- Create individual trade records for each transaction (linked via `trade_group_id`)
- Create transaction log entries in `position_transactions` table
- Calculate realized P&L when exits occur (FIFO or average cost basis)
- Update position quantities and average entry price
- Update unmatched transaction resolution to support hybrid pattern

**File: `backend/src/service/transform.rs`**

- Update transformation logic to:
- Create/update position records
- Create individual trade records linked via `trade_group_id`
- Log all transactions in `position_transactions` table
- Handle partial fills and multiple entries/exits

## Migration Strategy

1. **Options Schema Migration:**

- Create backup table
- Create new table structure
- Migrate data (map `total_premium` → `premium`, drop other columns)
- Update indexes
- Drop backup table

2. **Parent-Child Linking Migration:**

- Add new columns with NULL defaults
- For existing trades: generate `trade_group_id` (one per trade), set `quantity` from `number_shares`/`number_of_contracts`
- Existing trades become "parent" trades (no parent_trade_id)

3. **Position Tables Migration:**

- Create new position tables
- For existing trades: create position records, calculate aggregate quantities and average prices
- Create transaction log entries for all existing trades

## Testing Considerations

- Verify existing trades still work after migration
- Test creating parent trades and linking child trades
- Test trade group aggregation and P&L calculations
- Test position creation and updates from brokerage merge
- Test transaction log accuracy
- Test FIFO vs average cost basis for realized P&L
- Test partial exits and position reconciliation
- Verify indexes improve query performance

## Rollback Plan

- Keep backup tables during migration
- Schema version tracking allows rollback
- Data migration scripts should be reversible
- Position tables can be recalculated from transaction history if needed

### To-dos

- [ ] Remove unnecessary columns from options table (strategy_type, trade_direction, number_of_contracts, commissions, implied_volatility) and rename total_premium to premium
- [ ] Update OptionTrade model, DTOs, and all SQL queries to match new schema
- [ ] Add indexes for important columns in stocks and options tables (entry_price, exit_price, brokerage_name, strike_price for options)
- [ ] Add position_id, quantity, transaction_type, and parent_trade_id columns to stocks and options tables
- [ ] Update Stock model to support multiple entry/exit transactions with position tracking
- [ ] Update OptionTrade model to support multiple entry/exit transactions with position tracking
- [ ] Update brokerage merge logic to create separate entry/exit records for multiple transactions
- [ ] Create migration scripts to convert existing trades to new multi-entry/exit format