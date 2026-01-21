# OODA-247: SQL Injection Vulnerability Audit

## Observe

Audited SQL construction patterns in the storage crate.

### SQL Construction Patterns Found

| File            | Line                                      | Pattern           | Risk |
| --------------- | ----------------------------------------- | ----------------- | ---- |
| `kv.rs:120`     | `format!("SELECT...{}", self.table_name)` | LOW - Config-only |
| `kv.rs:138`     | `format!("SELECT...{}", self.table_name)` | LOW - Config-only |
| `kv.rs:157`     | `format!("SELECT...{}", self.table_name)` | LOW - Config-only |
| `kv.rs:227`     | `format!("SELECT...{}", self.table_name)` | LOW - Config-only |
| `kv.rs:240`     | `format!("SELECT...{}", self.table_name)` | LOW - Config-only |
| `vector.rs:620` | `format!("SELECT...{}", self.table_name)` | LOW - Config-only |

### Source of Table Names

```rust
// kv.rs:52
let table_name = format!("public.eq_{}_kv", prefix);
```

Where `prefix` comes from `PostgresConfig.table_prefix()` - server configuration, not user input.

### User Input Handling

All user inputs are properly parameterized:

```rust
// kv.rs:120-127
let sql = format!("SELECT value FROM {} WHERE key = $1", self.table_name);
let row: Option<(serde_json::Value,)> = sqlx::query_as(&sql)
    .bind(id)  // ✅ Parameterized binding
    .fetch_optional(&pool)
    .await
```

## Orient

### Security Analysis

| Aspect                  | Status | Notes                                     |
| ----------------------- | ------ | ----------------------------------------- |
| User input in SQL       | NONE   | All user data bound with `$1`, `$2`, etc. |
| Table names from config | ✅     | `PostgresConfig` is server-side           |
| Parameterized queries   | ✅     | All queries use sqlx bindings             |
| Dynamic SQL             | SAFE   | Only table names, from config             |

### Attack Vectors Considered

1. **User-provided table names**: NOT POSSIBLE

   - Table names come from `PostgresConfig`
   - Config is set at server startup

2. **User-provided query values**: MITIGATED

   - All values use `bind()` with `$1` placeholders
   - sqlx handles escaping automatically

3. **Namespace injection**: NOT POSSIBLE
   - Namespace comes from config, not user

## Decide

**Finding**: ✅ SQL injection is PROPERLY MITIGATED

**No changes needed** - all patterns follow parameterized query best practices.

## Act

Documented SQL injection audit as verified.

## Metrics

| Metric                 | Value |
| ---------------------- | ----- |
| Dynamic SQL locations  | 6     |
| User input in SQL      | 0     |
| Parameterized bindings | ALL   |
| Injection risk         | NONE  |

## Conclusion

✅ **SQL injection is MITIGATED**

All queries use parameterized bindings. Table names come from server configuration only.
