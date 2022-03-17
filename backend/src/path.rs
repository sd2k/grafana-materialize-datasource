//! Describes how targets should be represented in 'paths'
//! of Grafana Live channels.

use std::fmt::{self, Write};

use crate::queries::{Query, SelectStatement, SourceName, TailTarget};

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

impl PathDisplay for SourceName {
    fn fmt_path(&self, f: &mut String) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PathDisplay for SelectStatement {
    fn fmt_path(&self, f: &mut String) -> fmt::Result {
        f.write_str(QueryId::from_statement(self).as_str())
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct QueryId(String);

impl QueryId {
    pub fn new(s: String) -> QueryId {
        Self(s)
    }

    pub fn from_statement(statement: &SelectStatement) -> Self {
        Self(format!("{:x}", md5::compute(statement.as_str())))
    }

    pub fn into_inner(self) -> String {
        self.0
    }

    pub fn as_str(&self) -> &str {
        &self.0
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
                let query_id = QueryId::from_statement(statement);
                write!(f, "select/{}", &query_id.0)?;
            }
        }
        Ok(())
    }
}

impl PathDisplay for Query {
    fn fmt_path(&self, f: &mut String) -> fmt::Result {
        f.write_str("tail/")?;
        match self {
            Self::Tail(target) => target.fmt_path(f)?,
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_display() {
        assert_eq!(
            Query::Tail(TailTarget::Relation {
                name: "some_table".parse().unwrap()
            })
            .to_path(),
            "tail/relation/some_table"
        );
        assert_eq!(
            Query::Tail(TailTarget::Select {
                statement: "SELECT * FROM my_table".parse().unwrap()
            })
            .to_path(),
            "tail/select/9ebfce3b05a248842876e8ed1706a451"
        );
    }
}
