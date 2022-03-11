use std::{
    fmt::{self, Write},
    str::FromStr,
};

use serde::Deserialize;
use serde_with::DeserializeFromStr;
use tokio_postgres::{Client, Row, RowStream};

use crate::{Error, Result};

/// Trait describing how a type should be serialized to a [`Channel`]'s path.
///
/// Channel paths can only contain a alphanumeric + a few other characters,
/// so some types may need to encode their data differently.
///
/// [`Channel`]: grafana_plugin_sdk::live::Channel
pub trait PathDisplay {
    fn fmt_path(&self, f: &mut String) -> fmt::Result;
    fn to_path(&self) -> String {
        let mut s = String::new();
        self.fmt_path(&mut s)
            .expect("writing to a string must not fail");
        s
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, DeserializeFromStr)]
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

impl PathDisplay for SourceName {
    fn fmt_path(&self, f: &mut String) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for SourceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize)]
// TODO: actually do some validation here.
pub struct SelectStatement(String);

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

impl PathDisplay for TailTarget {
    fn fmt_path(&self, f: &mut String) -> fmt::Result {
        match self {
            Self::Relation { name } => {
                f.write_str("relation/")?;
                name.fmt_path(f)?;
            }
            Self::Select { statement } => {
                write!(f, "select/{}", base64::encode(statement.0.as_bytes()))?;
            }
        }
        Ok(())
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

impl PathDisplay for Path {
    fn fmt_path(&self, f: &mut String) -> fmt::Result {
        f.write_str("tail/")?;
        match self {
            Self::Tail(target) => target.fmt_path(f)?,
        };
        Ok(())
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tail(TailTarget::Relation { name }) => write!(f, "tail/relation/{}", name),
            Self::Tail(TailTarget::Select { statement }) => {
                write!(f, "tail/select/{}", statement)
            }
        }
    }
}

// Note that this differs from the `Deserialize` impl in that it assumes the SQL statement
// is base64 encoded - this should be tidied up at some point.
impl FromStr for Path {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut iter = s.splitn(3, '/');
        match (iter.next(), iter.next(), iter.next()) {
            (Some("tail"), Some("relation"), Some(name)) => Ok(Self::Tail(TailTarget::Relation {
                name: name.parse()?,
            })),
            (Some("tail"), Some("select"), Some(query_base64)) => {
                Ok(Self::Tail(TailTarget::Select {
                    statement: SelectStatement(
                        String::from_utf8(
                            base64::decode(&query_base64)
                                .map_err(|e| Error::InvalidTailTarget(e.to_string()))?,
                        )
                        .map_err(|e| Error::InvalidTailTarget(e.to_string()))?,
                    ),
                }))
            }
            (Some("tail"), _, _) => Err(Error::MissingTailTarget),
            _ => Err(Error::UnknownPath(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_relation() {
        assert_eq!(
            serde_json::from_str::<Path>(
                r#"{"path": "tail", "target": "relation", "name": "some_table"}"#
            )
            .unwrap(),
            Path::Tail(TailTarget::Relation {
                name: "some_table".parse().unwrap()
            })
        );
        assert!(serde_json::from_str::<Path>(
            r#"{"path": "tail", "target": "relation", "name": "little bobby tables"}"#
        )
        .is_err(),);
    }

    #[test]
    fn deserialize_statement() {
        assert_eq!(
            serde_json::from_str::<Path>(
                r#"{"path": "tail", "target": "select", "statement": "SELECT * FROM my_table"}"#
            )
            .unwrap(),
            Path::Tail(TailTarget::Select {
                statement: SelectStatement("SELECT * FROM my_table".to_string())
            })
        );
    }

    #[test]
    fn path_display() {
        assert_eq!(
            Path::Tail(TailTarget::Relation {
                name: SourceName("some_table".to_string())
            })
            .to_path(),
            "tail/relation/some_table"
        );
        assert_eq!(
            Path::Tail(TailTarget::Select {
                statement: SelectStatement("SELECT * FROM my_table".to_string())
            })
            .to_path(),
            "tail/select/U0VMRUNUICogRlJPTSBteV90YWJsZQ=="
        );
    }

    #[test]
    fn path_from_str() {
        assert_eq!(
            "tail/relation/some_table".parse::<Path>().unwrap(),
            Path::Tail(TailTarget::Relation {
                name: SourceName("some_table".to_string())
            })
        );
        assert_eq!(
            "tail/select/U0VMRUNUICogRlJPTSBteV90YWJsZQ=="
                .parse::<Path>()
                .unwrap(),
            Path::Tail(TailTarget::Select {
                statement: SelectStatement("SELECT * FROM my_table".to_string())
            })
        );
    }
}
