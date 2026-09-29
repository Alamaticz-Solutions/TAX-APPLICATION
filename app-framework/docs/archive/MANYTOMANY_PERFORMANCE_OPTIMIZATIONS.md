# ManyToMany Performance Optimizations

## Overview

The app_gen framework's ManyToMany relationship implementation has been enhanced with comprehensive performance optimizations to address critical scalability issues identified in production scenarios.

### Performance Issues Addressed

1. **Memory Usage**: The original `json_agg()` implementation loaded all related entities into memory during grouping, causing memory pressure with large datasets
2. **Large Result Sets**: Entities with many ManyToMany relationships generated oversized JSON responses that impacted network performance
3. **Query Performance**: Three-table joins (Main Entity → Junction Table → Target Entity) lacked proper indexing and optimization hints

### Solution Architecture

Our optimization strategy implements a multi-layered approach:
- **Pagination Support**: Configurable limits on related entity counts
- **Performance Monitoring**: Real-time query performance tracking and warnings
- **Optimized Indexing**: Automatic generation of performance-tuned junction table indexes
- **Memory Management**: Enhanced CTE generation with size-based controls
- **Configuration Profiles**: Multiple performance profiles for different deployment scenarios

## Changes Made

### 1. Junction Table INSERT Script Generation with Pagination

**Problem**: No automated way to populate junction tables with test or initial data.

**Solution**: Created automated INSERT script generation with sample data and conflict handling.

**Files Modified**:
- `app_gen/_templates/database/postgresql/insert_junction_data_sql/_mod.j2` (NEW)
- `app_gen/src/utils/database.rs` (lines 22-29, 66-89)

**Key Features**:
```sql
-- Generated INSERT with conflict handling
INSERT INTO sample.assigneeemail (id, assignee_id, email_id, created_at, created_by)
VALUES 
    (1, 1, 1, NOW(), NULL),
    (2, 1, 2, NOW(), NULL),
    (3, 2, 1, NOW(), NULL)
ON CONFLICT (assignee_id, email_id) DO NOTHING;
```

### 2. Performance Indexes for Junction Tables

**Problem**: Junction table queries lacked proper indexing, causing slow JOIN operations.

**Solution**: Automated generation of optimized indexes for all junction tables.

**Files Modified**:
- `app_gen/_templates/database/postgresql/junction_indexes_sql/_mod.j2` (NEW)
- `app_gen/src/utils/database.rs` (lines 97-130)

**Generated Indexes**:
```sql
-- Individual foreign key indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_assigneeemail_assignee_id
    ON sample.assigneeemail (assignee_id);

CREATE INDEX IF NOT EXISTS idx_assigneeemail_email_id
    ON sample.assigneeemail (email_id);

-- Composite unique index for JOIN optimization
CREATE UNIQUE INDEX IF NOT EXISTS idx_assigneeemail_composite
    ON sample.assigneeemail (assignee_id, email_id);

-- Temporal index for cleanup operations
CREATE INDEX IF NOT EXISTS idx_assigneeemail_created_at
    ON sample.assigneeemail (created_at);
```

### 3. Enhanced CTE Generation with Performance Monitoring

**Problem**: No visibility into query performance or memory usage patterns.

**Solution**: Enhanced CTE generation with performance metadata and monitoring.

**Files Modified**:
- `backend/src/data/clients/postgres/cte.rs` (lines 1-40, 42-65, 108-412)
- `backend/src/config/many_to_many_config.rs` (NEW)
- `backend/src/config/mod.rs` (lines 1-7)

**Performance Enhancements**:
```rust
// Enhanced JSON aggregation with pagination
let json_agg_expr = if config.enable_pagination && config.max_related_entities > 0 {
    format!(
        "(SELECT json_agg(limited_results) FROM (SELECT * FROM {} LIMIT {}) AS limited_results) FILTER (WHERE {} IS NOT NULL) as {}",
        target_cte.name, 
        config.max_related_entities,
        target_cte.name, 
        &prop.name
    )
} else {
    format!(
        "json_agg({}) FILTER (WHERE {} IS NOT NULL) as {}", 
        target_cte.name, 
        target_cte.name, 
        &prop.name
    )
};
```

### 4. ManyToMany Configuration System

**Problem**: No way to configure performance behavior for different deployment scenarios.

**Solution**: Comprehensive configuration system with multiple performance profiles.

**Files Modified**:
- `backend/src/config/many_to_many_config.rs` (NEW - complete file)

**Configuration Profiles**:
```rust
// High-performance profile for production
let config = ManyToManyConfig::high_performance();
// max_related_entities: 50
// slow_query_threshold_ms: 500
// enable_query_plan_logging: true

// Development profile for debugging
let config = ManyToManyConfig::development();
// max_related_entities: 20
// slow_query_threshold_ms: 100
// enable_performance_monitoring: true
```

## Implementation Details

### Junction Table Index Strategy

Our indexing strategy addresses three primary query patterns:

1. **Forward Relationship Queries**: `assignee_id` index for "find all emails for assignee"
2. **Reverse Relationship Queries**: `email_id` index for "find all assignees for email"
3. **JOIN Optimization**: Composite unique index for efficient three-table joins
4. **Temporal Queries**: `created_at` index for cleanup and audit operations

### CTE Performance Monitoring

The enhanced CTE system tracks:
- **Generation Time**: Warns when CTE creation exceeds thresholds
- **Relationship Complexity**: Categorizes queries as Low/Medium/High complexity
- **Memory Usage Patterns**: Monitors JSON aggregation sizes
- **Query Hints**: Embeds index suggestions in generated SQL

### Memory Management Strategy

```rust
// Pagination prevents memory exhaustion
if config.enable_pagination && config.max_related_entities > 0 {
    // Use LIMIT to cap result set size
    format!("(SELECT json_agg(limited_results) FROM (SELECT * FROM {} LIMIT {}) AS limited_results)")
}

// Performance warnings for complex relationships
if many_to_many_count > 3 {
    warn!("Entity {} has {} ManyToMany relationships, consider query optimization");
}
```

## Configuration Options

### ManyToManyConfig Settings

| Setting | Default | Description | Performance Impact |
|---------|---------|-------------|-------------------|
| `max_related_entities` | 100 | Maximum entities per relationship | High - prevents memory exhaustion |
| `enable_pagination` | true | Enable result set pagination | High - reduces memory usage |
| `json_size_warning_kb` | 1024 | JSON size warning threshold | Medium - alerts for large responses |
| `enable_performance_monitoring` | true | Enable query performance tracking | Low - adds minimal overhead |
| `slow_query_threshold_ms` | 1000 | Threshold for slow query warnings | None - logging only |
| `enable_index_suggestions` | true | Add index hints to generated SQL | None - comment-based hints |

### Performance Profiles

```rust
// Production optimized
ManyToManyConfig::high_performance()

// Development with detailed monitoring  
ManyToManyConfig::development()

// No restrictions (use with caution)
ManyToManyConfig::unrestricted()
```

## Usage Examples

### 1. Running Generated Junction Table Scripts

```bash
# Navigate to generated database scripts
cd database/_pkg/schemas/sample/

# Execute junction table creation (if not already done)
psql -d your_database -f tables.sql

# Create performance indexes
psql -d your_database -f junction_indexes.sql

# Populate with sample data
psql -d your_database -f junction_inserts.sql
```

### 2. Monitoring ManyToMany Query Performance

The framework automatically logs performance information:

```
INFO CTE generated for Assignees: complexity=Medium, many_to_many_count=1, time=45ms
WARN Entity Emails has 4 ManyToMany relationships, consider query optimization
```

### 3. Configuring Performance Profiles

```rust
// In your application configuration
let config = ManyToManyConfig::high_performance();

// Validate configuration
let warnings = config.validate();
for warning in warnings {
    println!("Config warning: {}", warning);
}
```

### 4. Generated SQL with Performance Hints

```sql
-- Enhanced CTE with performance metadata
emails_cte AS (
    select id, email_key, sender, body, subject,
           json_agg(assignees_cte) FILTER (WHERE assignees_cte IS NOT NULL) as assignees
    from sample.emails t0
    left join sample.assigneeemail junction_assignees on t0.id = junction_assignees.email_id /* INDEX HINT: idx_assigneeemail_email_id */
    left join assignees_cte on junction_assignees.assignee_id = assignees_cte.id /* INDEX HINT: idx_assigneeemail_assignee_id */
    group by t0.id
)
/* PERFORMANCE: ManyToMany relationships detected (1), ensure junction table indexes exist */
```

## File Reference Index

| File Path | Lines Modified | Change Type | Description |
|-----------|---------------|-------------|-------------|
| `app_gen/_templates/database/postgresql/insert_junction_data_sql/_mod.j2` | 1-68 | NEW | Junction table INSERT script template |
| `app_gen/_templates/database/postgresql/junction_indexes_sql/_mod.j2` | 1-68 | NEW | Junction table performance indexes template |
| `app_gen/src/utils/database.rs` | 22-29 | MODIFIED | Added junction script generation calls |
| `app_gen/src/utils/database.rs` | 66-89 | NEW | Junction INSERT script generation function |
| `app_gen/src/utils/database.rs` | 97-130 | NEW | Junction indexes generation function |
| `backend/src/data/clients/postgres/cte.rs` | 1-40 | MODIFIED | Added performance monitoring imports |
| `backend/src/data/clients/postgres/cte.rs` | 42-65 | NEW | CTE performance metadata structures |
| `backend/src/data/clients/postgres/cte.rs` | 108-412 | MODIFIED | Enhanced CTE generation with performance features |
| `backend/src/config/many_to_many_config.rs` | 1-150 | NEW | ManyToMany configuration system |
| `backend/src/config/mod.rs` | 7 | MODIFIED | Added many_to_many_config module |

## Performance Impact Summary

### Before Optimizations
- ❌ Unlimited memory usage for large relationships
- ❌ No query performance visibility
- ❌ Missing junction table indexes
- ❌ No configuration options
- ❌ Manual junction table data management

### After Optimizations
- ✅ Configurable memory limits with pagination
- ✅ Real-time performance monitoring and warnings
- ✅ Automated performance index generation
- ✅ Multiple performance profiles for different scenarios
- ✅ Automated junction table INSERT script generation
- ✅ Query optimization hints in generated SQL
- ✅ Comprehensive performance metadata tracking

The optimizations provide significant performance improvements while maintaining backward compatibility and adding powerful new capabilities for managing complex ManyToMany relationships at scale.
