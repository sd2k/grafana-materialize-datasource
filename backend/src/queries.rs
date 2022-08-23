//! Internal representations of queries requested by the frontend.

use grafana_plugin_sdk::live::Path;
use serde::Deserialize;
use serde_with::DeserializeFromStr;
use std::{fmt, str::FromStr};
use tokio_postgres::{Client, Row, RowStream};

use crate::{path, Error, Result, SqlQueries};

/// The name of a source the user wishes to tail.
///
/// This is a thin newtype wrapper around a string that
/// just does some very basic validation on creation.
#[derive(Clone, Debug, Hash, PartialEq, Eq, DeserializeFromStr)]
pub struct SourceName(String);

impl SourceName {
    /// Get the inner source name as a `&str`.
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

/// A select statement that the user wishes to tail.
///
/// This is a thin newtype wrapper around a string that
/// just does some very basic validation on creation.
#[derive(Clone, Debug, Hash, PartialEq, Eq, DeserializeFromStr)]
pub struct SelectStatement(String);

impl SelectStatement {
    /// Get the inner SQL statement as a `&str`.
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

/// The target of a `TAIL` query.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize)]
#[serde(tag = "target", rename_all = "camelCase")]
#[non_exhaustive]
pub enum TailTarget {
    /// Tail an existing relation (source, table or view).
    Relation { name: SourceName },
    /// Tail the output of a SELECT statement.
    Select { statement: SelectStatement },
}

impl TailTarget {
    /// `TAIL` this target using the provided client,
    /// returning a stream of rows from the target.
    ///
    /// Note that this runs `TAIL` with `SNAPSHOT = false` meaning
    /// that it does _not_ return a snapshot of the table immediately.
    /// This is because there is a many-to-one mapping between users
    /// and Grafana `run_stream` requests; only the first user to subscribe
    /// triggers `run_stream`, so we need to provide the initial data another
    /// way. See [`TailTarget::select_all`] for a method of doing so.
    pub async fn tail(&self, client: &Client) -> Result<RowStream> {
        let query = match self {
            Self::Relation { name } => format!("TAIL {name} WITH (SNAPSHOT = false)"),
            Self::Select { statement } => format!("TAIL ({statement}) WITH (SNAPSHOT = false)"),
        };
        let params: &[&str] = &[];
        Ok(client.query_raw(&query, params).await?)
    }

    /// Select all rows from this target into a `Vec`.
    ///
    /// This exists as a method of getting hold of the 'initial data'
    /// for a stream and should be called and returned to the user
    /// as part of their stream subscription (i.e. in `subscribe_stream`).
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

/// The query a user wishes to run.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "operation", rename_all = "camelCase")]
#[non_exhaustive]
pub enum Query {
    /// Tail the output of a relation.
    Tail(TailTarget),
}

impl Query {
    /// Try to convert a [`Path`] to a [`Query`], using the provided [`SqlQueries`]
    /// to lookup a query ID if the path contains one.
    ///
    /// # Errors
    ///
    /// This will fail if:
    /// - the path does not match a known format (`/tail/relation/<name>` or `/tail/select/<query id>`)
    /// - the query ID in the 'select' form is not present in `queries`
    pub async fn try_from_path(p: &Path, queries: SqlQueries) -> Result<Self> {
        let mut iter = p.as_str().splitn(3, '/');
        match (iter.next(), iter.next(), iter.next()) {
            (Some("tail"), Some("relation"), Some(name)) => Ok(Self::Tail(TailTarget::Relation {
                name: name.parse()?,
            })),
            (Some("tail"), Some("select"), Some(query_id)) => {
                let query_id = path::QueryId::new(query_id.to_string());
                Ok(Self::Tail(TailTarget::Select {
                    statement: queries
                        .read()
                        .await
                        .get(&query_id)
                        .cloned()
                        .ok_or_else(|| Error::InvalidTailTarget(query_id.into_inner()))?,
                }))
            }
            (Some("tail"), _, _) => Err(Error::MissingTailTarget),
            _ => Err(Error::UnknownPath(p.to_string())),
        }
    }

    /// Attempt to access this query as `&TailTarget`, or return an `Err` if it doesn't match.
    ///
    /// This is just a helper function to avoid having to match all over the place, since
    /// we know the query can't take any other form for now.
    pub(crate) fn as_tail(&self) -> Result<&TailTarget> {
        // If this enum changes in future we'll probably want to early return
        // hence using `match` instead of `if let`.
        match self {
            Self::Tail(target) => Ok(target),
            // This could change in future; don't want a catch-all
            // pattern though as we should handle it properly.
        }
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, sync::Arc};

    use tokio::sync::RwLock;

    use crate::path::QueryId;

    use super::*;

    #[test]
    fn deserialize_relation() {
        assert_eq!(
            serde_json::from_str::<Query>(
                r#"{"operation": "tail", "target": "relation", "name": "some_table"}"#
            )
            .unwrap(),
            Query::Tail(TailTarget::Relation {
                name: SourceName("some_table".parse().unwrap())
            })
        );
        assert!(serde_json::from_str::<Query>(
            r#"{"operation": "tail", "target": "relation", "name": "little bobby tables"}"#
        )
        .is_err(),);
    }

    #[test]
    fn deserialize_statement() {
        assert_eq!(
            serde_json::from_str::<Query>(
                r#"{"operation": "tail", "target": "select", "statement": "SELECT * FROM my_table"}"#
            )
            .unwrap(),
            Query::Tail(TailTarget::Select {
                statement: SelectStatement("SELECT * FROM my_table".parse().unwrap())
            })
        );
    }

    #[tokio::test]
    async fn query_from_str() {
        let queries = Arc::new(RwLock::new(HashMap::from([(
            QueryId::new("9ebfce3b05a248842876e8ed1706a451".to_string()),
            "SELECT * FROM my_table".parse().unwrap(),
        )])));
        assert_eq!(
            Query::try_from_path(
                &Path::new("tail/relation/some_table".to_string()).unwrap(),
                Arc::clone(&queries)
            )
            .await
            .unwrap(),
            Query::Tail(TailTarget::Relation {
                name: "some_table".parse().unwrap()
            })
        );
        assert_eq!(
            Query::try_from_path(
                &Path::new("tail/select/9ebfce3b05a248842876e8ed1706a451".to_string()).unwrap(),
                Arc::clone(&queries)
            )
            .await
            .unwrap(),
            Query::Tail(TailTarget::Select {
                statement: "SELECT * FROM my_table".parse().unwrap()
            })
        );
    }
}
