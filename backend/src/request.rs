/// Describes queries that can be made by Grafana to the 'query_data' handler.
use serde::Deserialize;

use crate::queries;

#[derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize)]
pub struct SourceName(queries::SourceName);

#[derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize)]
pub struct SelectStatement(queries::SelectStatement);

#[derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize)]
#[serde(tag = "target", rename_all = "camelCase")]
#[non_exhaustive]
pub enum TailTarget {
    /// Tail an existing relation (source, table or view).
    Relation { name: SourceName },
    /// Tail the output of a SELECT statement.
    Select { statement: SelectStatement },
}

impl Into<queries::TailTarget> for TailTarget {
    fn into(self) -> queries::TailTarget {
        match self {
            TailTarget::Relation { name } => queries::TailTarget::Relation { name: name.0 },
            TailTarget::Select { statement } => queries::TailTarget::Select {
                statement: statement.0,
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "path")]
#[non_exhaustive]
pub enum Path {
    /// Tail the output of a relation.
    #[serde(rename = "tail")]
    Tail(TailTarget),
}

impl Into<queries::Path> for Path {
    fn into(self) -> queries::Path {
        match self {
            Self::Tail(TailTarget::Relation { name }) => {
                queries::Path::Tail(queries::TailTarget::Relation { name: name.0 })
            }
            Self::Tail(TailTarget::Select { statement }) => {
                queries::Path::Tail(queries::TailTarget::Select {
                    statement: statement.0,
                })
            }
        }
    }
}

#[cfg(test)]
mod test {
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
}
