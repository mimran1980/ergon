# TableSchema — column management and schema diff

**Blocked by:** 00
**Blocks:** 03

The table schema type: holds column definitions, ORDER BY clause, and can diff
against a previous schema version for migration DDL generation.

## What to build

```rust
pub struct ColumnDef {
    pub name: String,
    pub col_type: ColumnType,
}

pub struct TableSchema {
    pub columns: Vec<ColumnDef>,
    pub order_by: Vec<String>,   // column names, default: ["_persist_time"]
    pub engine: TableEngine,     // MergeTree by default
}

pub struct SchemaDiff {
    pub new_columns: Vec<ColumnDef>,         // ADD COLUMN
    pub type_conflicts: Vec<TypeConflict>,   // incompatible, skip
    pub compatible_widens: Vec<TypeWiden>,   // ALTER COLUMN MODIFY TYPE
}
```

`TableSchema` must:
- Generate `CREATE TABLE` DDL
- Diff against another `TableSchema` → `SchemaDiff`
- Diff logic: match columns by name, detect new/missing/changed
- Type compatibility rules: widening only (u32→i64 OK, i32→u32 CONFLICT, u64→u32 CONFLICT). See DECISIONS.md for the agreed rules.

## Acceptance criteria

- [ ] `TableSchema` struct with columns, order_by, engine
- [ ] `TableSchema::diff(&self, previous: &TableSchema) -> SchemaDiff`
- [ ] `SchemaDiff` → DDL: `ALTER TABLE` statements for new columns and compatible widens
- [ ] Unit test: identical schemas → empty diff
- [ ] Unit test: new column added → one ADD COLUMN
- [ ] Unit test: u32→u64 → compatible widen (ALTER COLUMN MODIFY TYPE)
- [ ] Unit test: u64→u32 → type conflict (skipped)
- [ ] Unit test: i32→String → type conflict (skipped)
- [ ] Unit test: column removed → ignored (no DROP COLUMN)
- [ ] Unit test: full migration sequence — create, add 3 columns, 1 conflict, 2 compatible
- [ ] `_persist_time DateTime64(9)` auto-added if not explicitly declared
