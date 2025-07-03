use async_trait::async_trait;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use color_eyre::eyre::{self, Result};
use sqlx::{
  postgres::{PgColumn, PgPoolOptions, PgRow},
  sqlite::{SqliteColumn, SqliteRow},
  types::Uuid,
  Column, Row,
};
use tokio_stream::StreamExt as OtherStream;

use crate::{
  action::Action,
  app::dispatch,
  components::db::{DbColumn, DbTable},
};

#[async_trait]
pub trait Queryer: Send + Sync {
  async fn load_tables(&self, tx: tokio::sync::mpsc::UnboundedSender<Action>, search: &str) -> Result<()>;
  async fn load_table_columns(&self, table_name: &str, schema: &str) -> Result<Vec<DbColumn>>;
  async fn query(&self, query: &str, tx: tokio::sync::mpsc::UnboundedSender<Action>) -> Result<()>;
}

pub struct Sqlite {
  conn: sqlx::SqlitePool,
}

impl Sqlite {
  pub async fn new(filename: &str) -> Result<Self> {
    let conn = sqlx::SqlitePool::connect(&format!("sqlite:{filename}"))
      .await
      .map_err(|e| eyre::eyre!("Failed to connect to Sqlite: {}", e))?;

    Ok(Self { conn })
  }
}

pub struct Postgres {
  pool: sqlx::PgPool,
  database_name: String,
}

#[async_trait]
impl Queryer for Sqlite {
  async fn load_tables(&self, tx: tokio::sync::mpsc::UnboundedSender<Action>, search: &str) -> Result<()> {
    let table_query = r#"SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%'"#;
    let rows = sqlx::query(table_query).fetch_all(&self.conn).await?;

    let mut tables: Vec<DbTable> = rows
      .into_iter()
      .filter_map(|row| {
        row
          .try_get::<String, _>("name")
          .ok()
          .map(|name| DbTable { name, schema: "public".to_string(), columns: Vec::new() })
      })
      .collect();

    tables.sort_by(|a, b| a.name.cmp(&b.name));

    let filtered_tables = if search.is_empty() {
      tables
    } else {
      tables.into_iter().filter(|t| t.name.to_lowercase().contains(&search.to_lowercase())).collect()
    };

    dispatch(tx, Action::TablesLoaded(filtered_tables)).await?;
    Ok(())
  }

  async fn load_table_columns(&self, table_name: &str, _schema: &str) -> Result<Vec<DbColumn>> {
    let pragma_query = format!("PRAGMA table_info({table_name})");
    let rows = sqlx::query(&pragma_query).fetch_all(&self.conn).await?;

    let columns: Vec<DbColumn> = rows
      .into_iter()
      .filter_map(|row| {
        let name = row.try_get::<String, _>("name").ok()?;
        let type_str = row.try_get::<String, _>("type").ok()?;
        let not_null = row.try_get::<i32, _>("notnull").ok()?;
        let is_nullable = not_null == 0; // SQLite uses 0 for nullable, 1 for not null
        Some(DbColumn { name, data_type: type_str, is_nullable })
      })
      .collect();

    Ok(columns)
  }

  async fn query(&self, query: &str, tx: tokio::sync::mpsc::UnboundedSender<Action>) -> Result<()> {
    let mut rows = sqlx::query(query).fetch(&self.conn);

    let mut headers = vec![];
    let mut results = vec![];
    while let Some(row) = rows.try_next().await? {
      if headers.is_empty() {
        headers = row.columns().iter().map(|c| c.name().to_string()).collect();
      }
      let mut row_result = vec![];
      for c in row.columns() {
        if let Ok(v) = get_sqlite_value(&row, c) {
          row_result.push(v);
        }
      }

      results.push(row_result);
    }

    dispatch(tx, Action::QueryResult(headers, results)).await?;

    Ok(())
  }
}

impl Postgres {
  pub async fn new(conn_str: &str) -> Result<Self> {
    // let pool = sqlx::PgPool::connect("postgres://postgres:password@localhost:5432/postgres")
    let pool = PgPoolOptions::new().max_connections(5).connect(conn_str).await?;

    // Extract database name from connection string
    let database_name = if let Some(db_start) = conn_str.rfind('/') {
      let db_part = &conn_str[db_start + 1..];
      if let Some(query_start) = db_part.find('?') {
        db_part[..query_start].to_string()
      } else {
        db_part.to_string()
      }
    } else {
      "postgres".to_string()
    };

    Ok(Self { pool, database_name })
  }
}

#[async_trait]
impl Queryer for Postgres {
  async fn load_tables(&self, tx: tokio::sync::mpsc::UnboundedSender<Action>, search: &str) -> Result<()> {
    let rows = sqlx::query("SELECT table_name, table_schema FROM information_schema.tables WHERE table_catalog = $1")
      .bind(&self.database_name)
      .fetch_all(&self.pool)
      .await?;

    let mut tables: Vec<DbTable> = rows
      .into_iter()
      .filter_map(|row| {
        let name = row.try_get::<String, _>("table_name").ok()?;
        let schema = row.try_get::<String, _>("table_schema").ok()?;
        Some(DbTable { name, schema, columns: Vec::new() })
      })
      .collect();

    tables.sort_by(|a, b| a.name.cmp(&b.name));

    let filtered_tables = if search.is_empty() {
      tables
    } else {
      tables.into_iter().filter(|t| t.name.to_lowercase().contains(&search.to_lowercase())).collect()
    };

    dispatch(tx, Action::TablesLoaded(filtered_tables)).await?;
    Ok(())
  }

  async fn load_table_columns(&self, table_name: &str, schema: &str) -> Result<Vec<DbColumn>> {
    let rows = sqlx::query(
      "SELECT column_name, data_type, is_nullable 
       FROM information_schema.columns 
       WHERE table_catalog = $1 AND table_schema = $2 AND table_name = $3 
       ORDER BY ordinal_position",
    )
    .bind(&self.database_name)
    .bind(schema)
    .bind(table_name)
    .fetch_all(&self.pool)
    .await?;

    let columns: Vec<DbColumn> = rows
      .into_iter()
      .filter_map(|row| {
        let name = row.try_get::<String, _>("column_name").ok()?;
        let data_type = row.try_get::<String, _>("data_type").ok()?;
        let is_nullable_str = row.try_get::<String, _>("is_nullable").ok()?;
        let is_nullable = is_nullable_str == "YES";
        Some(DbColumn { name, data_type, is_nullable })
      })
      .collect();

    Ok(columns)
  }

  async fn query(&self, query: &str, tx: tokio::sync::mpsc::UnboundedSender<Action>) -> Result<()> {
    let mut rows = sqlx::query(query).fetch(&self.pool);

    let mut headers = vec![];
    let mut results = vec![];
    while let Some(row) = rows.try_next().await? {
      if headers.is_empty() {
        headers = row.columns().iter().map(|c| c.name().to_string()).collect();
      }
      let mut row_result = vec![];
      for c in row.columns() {
        if let Ok(v) = get_pg_value(&row, c) {
          row_result.push(v);
        }
      }

      results.push(row_result);
    }

    dispatch(tx, Action::QueryResult(headers, results)).await?;

    Ok(())
  }
}

#[macro_export]
macro_rules! get_or_null {
  ($value:expr) => {
    $value.map_or("NULL".to_string(), |v| v.to_string())
  };
}

fn get_sqlite_value(row: &SqliteRow, column: &SqliteColumn) -> Result<String> {
  let column_name = column.name();
  if let Ok(value) = row.try_get(column_name) {
    let value: Option<i16> = value;
    let v = value.map_or("NULL".to_string(), |v| v.to_string());
    Ok(v)
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<i32> = value;
    let v = value.map_or("NULL".to_string(), |v| v.to_string());
    Ok(v)
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<i64> = value;
    Ok(get_or_null!(value))
  // } else if let Ok(value) = row.try_get(column_name) {
  //   let value: Option<rust_decimal::Decimal> = value;
  //   Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: String = value;
    Ok(value)
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<NaiveDate> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: String = value;
    Ok(value)
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<chrono::DateTime<chrono::Utc>> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<chrono::DateTime<chrono::Local>> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<NaiveDateTime> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<NaiveDate> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<NaiveTime> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<serde_json::Value> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get::<Option<bool>, _>(column_name) {
    let value: Option<bool> = value;
    Ok(get_or_null!(value))
  // } else if let Ok(value) = row.try_get(column_name) {
  //   let value: Option<Vec<String>> = value;
  //   Ok(value.map_or("NULL".to_string(), |v| v.join(",")))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<Uuid> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<&[u8]> = value;
    Ok(value.map_or("NULL".to_string(), |values| {
      format!("\\x{}", values.iter().map(|v| format!("{v:02x}")).collect::<String>())
    }))
  } else {
    eyre::bail!("Unknown type for column {}", column_name);
  }
}

fn get_pg_value(row: &PgRow, column: &PgColumn) -> Result<String> {
  let column_name = column.name();
  if let Ok(value) = row.try_get(column_name) {
    let value: Option<i16> = value;
    let v = value.map_or("NULL".to_string(), |v| v.to_string());
    Ok(v)
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<i32> = value;
    let v = value.map_or("NULL".to_string(), |v| v.to_string());
    Ok(v)
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<i64> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<rust_decimal::Decimal> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: String = value;
    Ok(value)
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<NaiveDate> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: String = value;
    Ok(value)
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<chrono::DateTime<chrono::Utc>> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<chrono::DateTime<chrono::Local>> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<NaiveDateTime> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<NaiveDate> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<NaiveTime> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<serde_json::Value> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get::<Option<bool>, _>(column_name) {
    let value: Option<bool> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<Vec<String>> = value;
    Ok(value.map_or("NULL".to_string(), |v| v.join(",")))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<Uuid> = value;
    Ok(get_or_null!(value))
  } else if let Ok(value) = row.try_get(column_name) {
    let value: Option<&[u8]> = value;
    Ok(value.map_or("NULL".to_string(), |values| {
      format!("\\x{}", values.iter().map(|v| format!("{v:02x}")).collect::<String>())
    }))
  } else {
    eyre::bail!("Unknown type for column {}", column_name);
  }
}
