-- Migration: Add y_state column to notebook_notes table
-- Purpose: Store Y.js CRDT document state for real-time collaboration
-- Requirements: 2.1, 2.2

-- Add y_state column to store Y.js binary state
-- This column stores the CRDT state that enables real-time collaborative editing
-- The existing 'content' column continues to store Lexical JSON (derived from Y.js state)
ALTER TABLE notebook_notes ADD COLUMN y_state BLOB;

-- Note: SQLite doesn't support adding columns with constraints in ALTER TABLE
-- The column is nullable by default, which is correct for this use case
-- (notes created before collaboration was enabled won't have y_state)
