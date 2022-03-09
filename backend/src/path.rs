use std::{fmt, str::FromStr};

use serde::Deserialize;
use tokio_postgres::{Client, Row, RowStream};

use crate::{Error, Result};

#[derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize)]
pub struct SourceName(String);

impl FromStr for SourceName {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        todo!();
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
#[serde(tag = "target")]
#[non_exhaustive]
pub enum TailTarget {
    /// Tail an existing source, table or view.
    Object(SourceName),
    /// Tail the output of a SELECT statement.
    Select(SelectStatement),
}

impl TailTarget {
    pub async fn tail(&self, client: &Client) -> Result<RowStream> {
        let query = match self {
            Self::Object(name) => format!("TAIL {name} WITH (SNAPSHOT = false)"),
            Self::Select(statement) => format!("TAIL ({statement}) WITH (SNAPSHOT = false)"),
        };
        let params: &[&str] = &[];
        Ok(client.query_raw(&query, params).await?)
    }

    pub async fn select_all(&self, client: &Client) -> Result<Vec<Row>> {
        Ok(match self {
            Self::Object(name) => {
                client
                    .query(&format!("SELECT * FROM {}", name), &[])
                    .await?
            }
            Self::Select(statement) => client.query(&statement.0, &[]).await?,
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
            Self::Tail(TailTarget::Object(name)) => write!(f, "tail/object/{}", name),
            Self::Tail(TailTarget::Select(query)) => {
                write!(f, "tail/select/{}", query.to_path_segment())
            }
        }
    }
}

impl FromStr for Path {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut iter = s.splitn(3, '/');
        match (iter.next(), iter.next(), iter.next()) {
            (Some("tail"), Some("object"), Some(name)) => {
                Ok(Self::Tail(TailTarget::Object(name.parse()?)))
            }
            (Some("tail"), Some("select"), Some(query)) => Ok(Self::Tail(TailTarget::Select(
                SelectStatement::from_path_segment(query)?,
            ))),
            (Some("tail"), _, _) => Err(Error::MissingTailTarget),
            _ => Err(Error::UnknownPath(s.to_string())),
        }
    }
}

// TODO(bsull): fix these once we know what the path should look like as JSON.
// #[cfg(test)]
// mod tests {
//     use super::*;
//
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
