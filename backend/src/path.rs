use std::{
    fmt::{self, Write},
    str::FromStr,
};

use crate::{queries, Error, Result};

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

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct SourceName(queries::SourceName);

impl Into<queries::SourceName> for SourceName {
    fn into(self) -> queries::SourceName {
        self.0
    }
}

impl PathDisplay for SourceName {
    fn fmt_path(&self, f: &mut String) -> fmt::Result {
        f.write_str(self.0.as_str())
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
        Ok(Self(s.parse()?))
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
// TODO: actually do some validation here.
pub struct SelectStatement(queries::SelectStatement);

impl fmt::Display for SelectStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct QueryId(String);

impl QueryId {
    fn from_statement(statement: &queries::SelectStatement) -> Self {
        Self(format!("{:x}", md5::compute(statement.as_str())))
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum TailTarget {
    /// Tail an existing relation (source, table or view).
    Relation { name: SourceName },
    /// Tail the output of a SELECT statement.
    Select { query_id: QueryId },
}

impl PathDisplay for TailTarget {
    fn fmt_path(&self, f: &mut String) -> fmt::Result {
        match self {
            Self::Relation { name } => {
                f.write_str("relation/")?;
                name.fmt_path(f)?;
            }
            Self::Select { query_id } => {
                write!(f, "select/{}", &query_id.0)?;
            }
        }
        Ok(())
    }
}

impl From<queries::TailTarget> for TailTarget {
    fn from(other: queries::TailTarget) -> Self {
        match other {
            queries::TailTarget::Relation { name } => Self::Relation {
                name: SourceName(name),
            },
            queries::TailTarget::Select { statement } => Self::Select {
                query_id: QueryId::from_statement(&statement),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Path {
    /// Tail the output of a relation.
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
            (Some("tail"), Some("select"), Some(query_id)) => Ok(Self::Tail(TailTarget::Select {
                query_id: QueryId(query_id.to_string()),
            })),
            (Some("tail"), _, _) => Err(Error::MissingTailTarget),
            _ => Err(Error::UnknownPath(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_display() {
        assert_eq!(
            Path::Tail(TailTarget::Relation {
                name: SourceName("some_table".parse().unwrap())
            })
            .to_path(),
            "tail/relation/some_table"
        );
        assert_eq!(
            Path::Tail(TailTarget::Select {
                query_id: QueryId::from_statement(&"SELECT * FROM my_table".parse().unwrap())
            })
            .to_path(),
            "tail/select/9ebfce3b05a248842876e8ed1706a451"
        );
    }

    #[test]
    fn path_from_str() {
        assert_eq!(
            "tail/relation/some_table".parse::<Path>().unwrap(),
            Path::Tail(TailTarget::Relation {
                name: SourceName("some_table".parse().unwrap())
            })
        );
        assert_eq!(
            "tail/select/9ebfce3b05a248842876e8ed1706a451"
                .parse::<Path>()
                .unwrap(),
            Path::Tail(TailTarget::Select {
                query_id: QueryId::from_statement(&"SELECT * FROM my_table".parse().unwrap())
            })
        );
    }
}
