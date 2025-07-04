use query_crafter::components::db::{DbTable, DbColumn};
use color_eyre::Result;
use std::sync::Arc;
use tempfile::NamedTempFile;

pub async fn create_test_db() -> Result<Arc<dyn query_crafter::sql::Queryer>> {
    // Create a temporary file for the SQLite database
    let temp_file = NamedTempFile::new()?;
    let db_path = temp_file.path().to_str().unwrap().to_string();
    
    // Create the database
    let db = query_crafter::sql::Sqlite::new(&db_path).await?;
    
    // We can't directly execute SQL on the Queryer trait,
    // so we'll return the empty database
    // The tests that need data will need to use the query method
    
    Ok(Arc::new(db))
}

pub fn sample_tables() -> Vec<DbTable> {
    vec![
        DbTable {
            name: "users".to_string(),
            schema: "public".to_string(),
            columns: vec![],
        },
        DbTable {
            name: "posts".to_string(),
            schema: "public".to_string(),
            columns: vec![],
        },
        DbTable {
            name: "comments".to_string(),
            schema: "public".to_string(),
            columns: vec![],
        },
    ]
}

pub fn sample_columns() -> Vec<DbColumn> {
    vec![
        DbColumn {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            is_nullable: false,
        },
        DbColumn {
            name: "name".to_string(),
            data_type: "TEXT".to_string(),
            is_nullable: false,
        },
        DbColumn {
            name: "email".to_string(),
            data_type: "TEXT".to_string(),
            is_nullable: false,
        },
    ]
}

pub fn sample_query_results() -> (Vec<String>, Vec<Vec<String>>) {
    let headers = vec![
        "id".to_string(),
        "name".to_string(),
        "email".to_string(),
    ];
    
    let results = vec![
        vec!["1".to_string(), "Alice Johnson".to_string(), "alice@example.com".to_string()],
        vec!["2".to_string(), "Bob Smith".to_string(), "bob@example.com".to_string()],
        vec!["3".to_string(), "Charlie Brown".to_string(), "charlie@example.com".to_string()],
    ];
    
    (headers, results)
}

pub fn long_query_results() -> (Vec<String>, Vec<Vec<String>>) {
    let headers = vec![
        "id".to_string(),
        "title".to_string(),
        "author".to_string(),
        "status".to_string(),
        "created_at".to_string(),
    ];
    
    let mut results = Vec::new();
    for i in 1..=50 {
        results.push(vec![
            i.to_string(),
            format!("Post Title {}", i),
            format!("Author {}", i % 3 + 1),
            if i % 4 == 0 { "draft" } else { "published" }.to_string(),
            "2024-01-01 12:00:00".to_string(),
        ]);
    }
    
    (headers, results)
}

pub fn unformatted_query() -> &'static str {
    "select u.id,u.name,u.email,count(p.id) as post_count from users u left join posts p on u.id=p.user_id where p.published=true group by u.id,u.name,u.email order by post_count desc"
}

pub fn formatted_query() -> &'static str {
    "SELECT
  u.id,
  u.name,
  u.email,
  COUNT(p.id) AS post_count
FROM
  users u
  LEFT JOIN posts p ON u.id = p.user_id
WHERE
  p.published = TRUE
GROUP BY
  u.id,
  u.name,
  u.email
ORDER BY
  post_count DESC"
}