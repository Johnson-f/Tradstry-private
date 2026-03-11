<!-- 14942b76-92a9-401e-96e5-95839af1bb5f 06e51058-73ed-4fc0-9153-5658b95d36f5 -->
# Redesign Caching Service Architecture

## Overview

Split the monolithic `cache_service.rs` (541 lines) into a modular structure within `backend/src/service/caching/` folder. The main `CacheService` will act as a facade that delegates to specialized services.

## File Structure

### Core Files

1. **`mod.rs`** - Module exports and CacheService facade
2. **`types.rs`** - Shared types (CacheStats, etc.)
3. **`config.rs`** - Configuration (TTL mappings, cacheable tables, table TTL getters)

### Service Modules

4. **`preloader.rs`** - User data preloading logic

- `preload_user_data()` - Main preload orchestration
- `preload_table_data()` - Individual table preloading
- `preload_analytics()` - Analytics preloading

5. **`query_builder.rs`** - Dynamic query construction

- `build_select_query()` - Query building with table-specific ordering
- `build_select_query_with_validation()` - Query building with column validation
- `table_has_column()` - Column existence checking

6. **`converter.rs`** - Data conversion utilities

- `row_to_json()` - Convert database rows to JSON
- `get_column_value()` - Type-safe column value extraction

7. **`invalidator.rs`** - Cache invalidation operations

- `invalidate_pattern()` - Pattern-based invalidation
- `invalidate_user_cache()` - User-level invalidation
- `invalidate_table_cache()` - Table-level invalidation
- `invalidate_user_analytics()` - Analytics invalidation

8. **`analytics.rs`** - Analytics calculation and caching

- `calculate_table_analytics()` - Analytics computation
- `preload_analytics()` - Analytics preloading logic

9. **`health.rs`** - Health checks and statistics

- `health_check()` - Service health verification
- `get_cache_stats()` - Cache statistics retrieval

10. **`cache_operations.rs`** - Core cache operations

- `get_or_fetch()` - Cache-aside pattern implementation

## Implementation Details

### Module Organization

- Each module will be self-contained with its own error handling
- Shared dependencies (RedisClient, TableSchema) will be passed as parameters
- The facade pattern allows CacheService to maintain the same public API

### Key Changes

- Extract schema_cache management to a separate `SchemaManager` or keep in CacheService
- Move TTL configuration logic to `config.rs`
- Separate concerns: preloading, querying, conversion, invalidation
- Maintain backward compatibility with existing route handlers

### Dependencies

- `RedisClient` - Passed to each service module
- `TableSchema` - Used for schema-aware operations
- `Connection` - Database connection for preloading operations

## Migration Strategy

1. Create new module files in `caching/` folder
2. Move code from `cache_service.rs` to appropriate modules
3. Update `CacheService` to use new modules
4. Update `service/mod.rs` to export from `caching` module
5. Verify all route handlers continue to work with new structure
6. Remove old `cache_service.rs` file

## Benefits

- Better code organization and maintainability
- Easier testing of individual components
- Clear separation of concerns
- Follows existing patterns (ai_service, analytics_engine)
- Reduced cognitive load per file

### To-dos

- [ ] Create types.rs with CacheStats and other shared types
- [ ] Create config.rs with TTL mappings, cacheable tables list, and table TTL getter functions
- [ ] Create query_builder.rs with dynamic query construction and column validation logic
- [ ] Create converter.rs with row_to_json and get_column_value functions
- [ ] Create invalidator.rs with all cache invalidation pattern methods
- [ ] Create analytics.rs with analytics calculation and preloading logic
- [ ] Create health.rs with health_check and get_cache_stats methods
- [ ] Create cache_operations.rs with get_or_fetch cache-aside pattern
- [ ] Create preloader.rs with user data preloading, table preloading, and analytics preloading
- [ ] Create mod.rs with CacheService facade that delegates to specialized services
- [ ] Update service/mod.rs to export caching module instead of cache_service
- [ ] Update all route handlers and services to import from new caching module location
- [ ] Remove old cache_service.rs file after migration is complete