use rusqlite::{Connection, Params, Row};
use thiserror::Error;

pub mod metrics;
pub mod operational_data;
pub mod plex_db;

fn query_and_map<T, P, F>(
    conn: &Connection,
    title: &str,
    sql: &str,
    params: P,
    row_map_fn: F,
) -> Result<Vec<T>, DbErr>
where
    P: Params,
    F: FnMut(&Row<'_>) -> rusqlite::Result<T>,
{
    let mut query = conn
        .prepare(sql)
        .map_err(|e| DbErr::PrepSqlFailure(format!("error preparing query for {title}: {e}")))?;
    let results = query
        .query_map(params, row_map_fn)
        .map_err(|e| {
            DbErr::HandleQueryResultFailure(format!("error handling query for {title}: {e}"))
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| DbErr::ParseIssue(format!("error parsing result for {title}: {e}")));
    results
}

#[derive(Error, Debug)]
pub enum DbErr {
    #[error("failed to prepare sql statement: {0}")]
    PrepSqlFailure(String),
    #[error("Failed to handle results: {0}")]
    HandleQueryResultFailure(String),
    #[error("Failed to parse result: {0}")]
    ParseIssue(String),
    #[error("no results")]
    NoResults,
}
