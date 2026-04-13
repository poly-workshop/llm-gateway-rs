/// Database pool abstraction supporting both PostgreSQL and SQLite.
///
/// When `DATABASE_URL` starts with `sqlite:`, a SQLite pool is created;
/// otherwise a PostgreSQL pool is used.
#[derive(Clone)]
pub enum DbPool {
    Pg(sqlx::PgPool),
    Sqlite(sqlx::SqlitePool),
}

impl DbPool {
    pub fn is_sqlite(&self) -> bool {
        matches!(self, DbPool::Sqlite(_))
    }
}

// ── Query dispatch macros ─────────────────────────────────────────────
//
// These macros duplicate the query call across both pool variants so that
// callers can write database-agnostic service code.  The Rust compiler
// resolves both arms at compile time, and only the active variant runs.

/// `db_query_as!(mode, pool, sql [, bind]*)` — run a `query_as` against DbPool.
///
/// Modes:
///   - `optional` — `fetch_optional`, returns `Result<Option<T>, sqlx::Error>`
///   - `all`      — `fetch_all`, returns `Result<Vec<T>, sqlx::Error>`
///   - `one`      — `fetch_one`, returns `Result<T, sqlx::Error>` (errors if no row)
macro_rules! db_query_as {
    (optional, $pool:expr, $sql:expr $(, $bind:expr)*) => {{
        match $pool {
            $crate::db::DbPool::Pg(p) => {
                sqlx::query_as($sql)
                    $(.bind($bind))*
                    .fetch_optional(p)
                    .await
            }
            $crate::db::DbPool::Sqlite(p) => {
                sqlx::query_as($sql)
                    $(.bind($bind))*
                    .fetch_optional(p)
                    .await
            }
        }
    }};
    (all, $pool:expr, $sql:expr $(, $bind:expr)*) => {{
        match $pool {
            $crate::db::DbPool::Pg(p) => {
                sqlx::query_as($sql)
                    $(.bind($bind))*
                    .fetch_all(p)
                    .await
            }
            $crate::db::DbPool::Sqlite(p) => {
                sqlx::query_as($sql)
                    $(.bind($bind))*
                    .fetch_all(p)
                    .await
            }
        }
    }};
    (one, $pool:expr, $sql:expr $(, $bind:expr)*) => {{
        match $pool {
            $crate::db::DbPool::Pg(p) => {
                sqlx::query_as($sql)
                    $(.bind($bind))*
                    .fetch_one(p)
                    .await
            }
            $crate::db::DbPool::Sqlite(p) => {
                sqlx::query_as($sql)
                    $(.bind($bind))*
                    .fetch_one(p)
                    .await
            }
        }
    }};
}

/// `db_execute!(pool, sql [, bind]*)` — execute a statement, returns `Result<u64, sqlx::Error>`.
macro_rules! db_execute {
    ($pool:expr, $sql:expr $(, $bind:expr)*) => {{
        match $pool {
            $crate::db::DbPool::Pg(p) => {
                sqlx::query($sql)
                    $(.bind($bind))*
                    .execute(p)
                    .await
                    .map(|r| r.rows_affected())
            }
            $crate::db::DbPool::Sqlite(p) => {
                sqlx::query($sql)
                    $(.bind($bind))*
                    .execute(p)
                    .await
                    .map(|r| r.rows_affected())
            }
        }
    }};
}

/// `db_query_scalar!(mode, pool, sql [, bind]*)` — run a `query_scalar` against DbPool.
///
/// `mode` is one of `optional`, `all`, `one`.
macro_rules! db_query_scalar {
    (optional, $pool:expr, $sql:expr $(, $bind:expr)*) => {{
        match $pool {
            $crate::db::DbPool::Pg(p) => {
                sqlx::query_scalar($sql)
                    $(.bind($bind))*
                    .fetch_optional(p)
                    .await
            }
            $crate::db::DbPool::Sqlite(p) => {
                sqlx::query_scalar($sql)
                    $(.bind($bind))*
                    .fetch_optional(p)
                    .await
            }
        }
    }};
    (all, $pool:expr, $sql:expr $(, $bind:expr)*) => {{
        match $pool {
            $crate::db::DbPool::Pg(p) => {
                sqlx::query_scalar($sql)
                    $(.bind($bind))*
                    .fetch_all(p)
                    .await
            }
            $crate::db::DbPool::Sqlite(p) => {
                sqlx::query_scalar($sql)
                    $(.bind($bind))*
                    .fetch_all(p)
                    .await
            }
        }
    }};
    (one, $pool:expr, $sql:expr $(, $bind:expr)*) => {{
        match $pool {
            $crate::db::DbPool::Pg(p) => {
                sqlx::query_scalar($sql)
                    $(.bind($bind))*
                    .fetch_one(p)
                    .await
            }
            $crate::db::DbPool::Sqlite(p) => {
                sqlx::query_scalar($sql)
                    $(.bind($bind))*
                    .fetch_one(p)
                    .await
            }
        }
    }};
}
