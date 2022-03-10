use std::{fmt, str::FromStr};

use serde::Deserialize;
use tokio_postgres::{Client, Row, RowStream};

use crate::{Error, Result};

#[derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize)]
pub struct SourceName(String);

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

impl fmt::Display for SourceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize)]
pub struct SelectStatement(String);

impl SelectStatement {
    fn to_path_segment(&self) -> String {
        todo!()
    }

    fn from_path_segment(path_segment: &str) -> Result<Self> {
        todo!()
    }
}

impl fmt::Display for SelectStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug, Deserialize, Hash, PartialEq, Eq)]
#[serde(tag = "target", rename_all = "camelCase")]
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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "path")]
#[non_exhaustive]
pub enum Path {
    /// Tail the output of a relation.
    #[serde(rename = "tail")]
    Tail(TailTarget),
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tail(TailTarget::Relation { name }) => write!(f, "tail/relation/{}", name),
            Self::Tail(TailTarget::Select { statement }) => {
                write!(f, "tail/select/{}", statement.to_path_segment())
            }
        }
    }
}

impl FromStr for Path {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut iter = s.splitn(3, '/');
        match (iter.next(), iter.next(), iter.next()) {
            (Some("tail"), Some("relation"), Some(name)) => Ok(Self::Tail(TailTarget::Relation {
                name: name.parse()?,
            })),
            (Some("tail"), Some("select"), Some(query)) => Ok(Self::Tail(TailTarget::Select {
                statement: SelectStatement::from_path_segment(query)?,
            })),
            (Some("tail"), _, _) => Err(Error::MissingTailTarget),
            _ => Err(Error::UnknownPath(s.to_string())),
        }
    }
}

// TODO(bsull): fix these once we know what the path should look like as JSON.
// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn deserialize_path() {
//         assert_eq!(
//             serde_json::from_str::<Path>(r#"{"path": "tasks"}"#).unwrap(),
//             Path::Tasks
//         );
//         assert_eq!(
//             serde_json::from_str::<Path>(r#"{"path": "task", "taskId": 1}"#).unwrap(),
//             Path::TaskDetails { task_id: TaskId(1) }
//         );
//     }
// }
