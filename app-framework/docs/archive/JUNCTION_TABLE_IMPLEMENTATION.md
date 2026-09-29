# Junction Table Implementation

This document describes the automatic junction table management implementation for many-to-many relationships in the PostgreSQL client, including recent fixes for datetime validation and SQL parameter handling.

## Overview

The system now automatically handles junction table operations for many-to-many relationships during entity create, update, and delete operations. All operations are wrapped in database transactions to ensure atomicity.

## Features Implemented

### 1. Automatic Junction Table Insertion
- When creating an entity with many-to-many relationships, junction table records are automatically created
- Uses database transactions to ensure atomicity between main entity and junction table operations
- Handles constraint violations gracefully with `ON CONFLICT DO NOTHING`

### 2. Junction Table Updates
- When updating an entity, existing junction table records are replaced with new ones
- Uses a delete-then-insert strategy within a transaction
- Maintains referential integrity throughout the process

### 3. Cascade Deletion
- When deleting an entity, all related junction table records are automatically removed
- Prevents orphaned junction table records
- Performed within a transaction for consistency

### 4. Comprehensive Error Handling
- Detailed error messages for constraint violations
- Transaction rollback on any failure
- Validation of related entity IDs
- Logging of junction table operations for debugging

## Usage Examples

### Creating an Entity with Many-to-Many Relationships

```rust
// Input JSON with many-to-many relationships
let input = json!({
    "first_name": "John",
    "last_name": "Doe",
    "emails": [
        {"id": 1},  // Existing email with ID 1
        {"id": 2}   // Existing email with ID 2
    ]
});

// The system will:
// 1. Insert the assignee record
// 2. Insert junction records linking assignee to emails
// 3. Commit the transaction
let result = postgres_client.create_item_json(
    entity_type,
    selections,
    input.as_object().unwrap().clone(),
    &user,
    &access
).await?;
```

### Updating Many-to-Many Relationships

```rust
// Update with new email relationships
let input = json!({
    "id": 1,
    "first_name": "John",
    "last_name": "Smith",  // Updated name
    "emails": [
        {"id": 2},  // Keep email 2
        {"id": 3}   // Add email 3, remove email 1
    ]
});

// The system will:
// 1. Update the assignee record
// 2. Delete all existing junction records for this assignee
// 3. Insert new junction records for emails 2 and 3
// 4. Commit the transaction
```

### Deleting an Entity

```rust
let input = json!({"id": 1});

// The system will:
// 1. Delete all junction table records for this entity
// 2. Delete the main entity record
// 3. Commit the transaction
let result = postgres_client.delete_item_json(
    entity_type,
    input.as_object().unwrap().clone(),
    &user,
    &access,
    None
).await?;
```

## Configuration Requirements

### Entity Type Configuration

Your entity type must have many-to-many properties configured:

```yaml
- name: emails
  caption: Assigned Emails
  data_type: ManyToMany
  many_to_many_property:
    junction_table: AssigneeEmail
    junction_schema: "sample"  # optional, defaults to current schema
    local_key: assignee_id     # FK to current entity
    foreign_key: email_id      # FK to target entity
    target_schema: "sample"
    target_type: Emails
```

### Junction Table Schema

The junction table must exist with the following structure:

```sql
CREATE TABLE sample.assigneeemail (
    "id" bigint NOT NULL PRIMARY KEY,
    "assignee_id" bigint NOT NULL,
    "email_id" bigint NOT NULL,
    "created_at" timestamp DEFAULT NOW(),
    "created_by" bigint,
    UNIQUE("assignee_id", "email_id")
);
```

## Error Handling

The implementation includes comprehensive error handling:

### Validation Errors
- Invalid entity ID formats
- Missing required fields
- Malformed related entity references

### Database Errors
- Foreign key constraint violations
- Unique constraint violations
- Transaction failures

### Example Error Messages
```
"Invalid related entity at index 0: Entity ID must be a number or string"
"Failed to insert junction record for sample.assigneeemail: duplicate key value"
"Failed to cascade delete junction records for sample.assigneeemail: relation does not exist"
```

## Transaction Behavior

All junction table operations are wrapped in database transactions:

1. **Create**: Main entity insert + junction inserts in one transaction
2. **Update**: Main entity update + junction delete/insert in one transaction  
3. **Delete**: Junction deletes + main entity delete in one transaction

If any operation fails, the entire transaction is rolled back, ensuring data consistency.

## Performance Considerations

- Junction table operations add overhead to create/update/delete operations
- Use appropriate indexes on junction tables (automatically generated)
- Consider batch operations for large numbers of relationships
- Monitor transaction log size for high-volume operations

## Debugging

Enable detailed logging to see junction table operations:

```rust
println!("Successfully inserted {} junction records for {}.{}", 
         related_entities.len(), junction_schema, junction_table);
```

Log messages include:
- Number of records inserted/updated/deleted
- Junction table names and schemas
- Error details for failed operations
- Transaction commit/rollback status

## Limitations

1. **Related Entity Validation**: The system doesn't validate that related entities exist
2. **Circular References**: No protection against circular many-to-many relationships
3. **Bulk Operations**: No optimized bulk insert/update for large relationship sets
4. **Soft Deletes**: Junction records are hard deleted, not soft deleted

## Recent Fixes (2025-08-01)

### DateTime Validation Error Resolution

Fixed critical datetime validation errors that were preventing entity creation with computed properties and auto-generated timestamps.

#### Issues Resolved

1. **Missing Chrono Features**: The `chrono` dependency was missing required features for datetime serialization
2. **SQL Parameter Bug**: Number filter criteria were generating invalid SQL with missing parameter placeholders
3. **ID Type Conversion**: String-to-number conversion was failing for database queries

#### Files Modified

**1. backend/Cargo.toml**
```toml
# Added missing chrono features for datetime serialization
chrono = { version = "0.4", features = ["serde", "clock"] }
```
*Why*: The `serde` feature enables JSON serialization of datetime values, and `clock` provides current time functionality for `DateTimeNow` computed properties.

**2. backend/src/data/clients/postgres/filter.rs**
```rust
// Fixed missing $ in parameter placeholder (line 498)
// Before:
let res = format!("{}.{} {} {}", alias, prop.name, oper, params.len());
// After:
let res = format!("{}.{} {} ${}", alias, prop.name, oper, params.len());
```
*Why*: PostgreSQL requires parameter placeholders to be prefixed with `$` (e.g., `$1`, `$2`). The missing `$` caused SQL parameter count mismatches.

**3. backend/src/data/clients/postgres/postgres_client.rs**
```rust
// Added proper ID type conversion for database queries (lines 900-907)
let id_value = if let Ok(int_id) = id.parse::<i64>() {
  serde_json::Value::Number(serde_json::Number::from(int_id))
} else {
  serde_json::Value::String(id)
};
let filter = Some(serde_json::json!({ &pk_name: id_value }));
```
*Why*: The database expects integer IDs as numbers, not strings. This conversion ensures proper type matching for primary key lookups.

#### Impact

These fixes resolved the following error sequence:
1. ✅ **Computed Properties**: `full_name` concatenation now works correctly
2. ✅ **DateTime Generation**: `created_at` timestamps are properly generated and serialized
3. ✅ **Database Operations**: Entity creation, insertion, and retrieval all work seamlessly
4. ✅ **API Responses**: Complete entity data is returned with all requested fields

#### Testing

The fix was validated with successful entity creation:
```bash
curl -X POST http://127.0.0.1:3000/sample \
  -H "Content-Type: application/json" \
  -H "Authorization: <redacted>" \
  -d '{
    "query": "mutation CreateAssignee { createAssignees(input: { email: \"test@company.com\", first_name: \"Test\", last_name: \"User\", status: Active }) { id first_name last_name email } }"
  }'

# Response:
{
  "data": {
    "createAssignees": {
      "id": 8,
      "first_name": "Test",
      "last_name": "User",
      "email": "test@company.com"
    }
  }
}
```

## Future Enhancements

- Batch junction table operations for better performance
- Related entity existence validation
- Soft delete support for junction records
- Audit trail for junction table changes
- Optimistic concurrency control for junction records
