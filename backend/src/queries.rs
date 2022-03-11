//! Describes queries that can be made by Grafana to the 'query_data' handler.
use serde_with::DeserializeFromStr;
use std::{fmt, str::FromStr};
use tokio_postgres::{Client, Row, RowStream};

use crate::{Error, Result};

#[derive(Clone, Debug, Hash, PartialEq, Eq, DeserializeFromStr)]
pub struct SourceName(String);

impl SourceName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SourceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for SourceName {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        if s.find(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '_'))
            .is_some()
        {
            Err(Error::InvalidTailTarget(format!(
                "Invalid relation name {s}"
            )))
        } else {
            Ok(Self(s.to_string()))
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, DeserializeFromStr)]
pub struct SelectStatement(String);

impl SelectStatement {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for SelectStatement {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        // TODO: actually validate here.
        Ok(Self(s.to_string()))
    }
}

impl fmt::Display for SelectStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum TailTarget {
    /// Tail an existing relation (source, table or view).
    Relation { name: SourceName },
    /// Tail the output of a SELECT statement.
    Select { statement: SelectStatement },
}

impl TailTarget {
    pub async fn tail(&self, client: &Client) -> Result<RowStream> {
        let query = match self {
            Self::Relation { name } => format!("TAIL {name} WITH (SNAPSHOT = false)"),
            Self::Select { statement } => format!("TAIL ({statement}) WITH (SNAPSHOT = false)"),
        };
        let params: &[&str] = &[];
        Ok(client.query_raw(&query, params).await?)
    }

    pub async fn select_all(&self, client: &Client) -> Result<Vec<Row>> {
        Ok(match self {
            Self::Relation { name } => {
                client
                    .query(&format!("SELECT * FROM {}", name), &[])
                    .await?
            }
            Self::Select { statement } => client.query(&statement.0, &[]).await?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Path {
    /// Tail the output of a relation.
    Tail(TailTarget),
}
