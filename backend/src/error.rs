use grafana_plugin_sdk::{backend, data, live};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("stream already running")]
    StreamAlreadyRunning,

    #[error("invalid query")]
    InvalidQuery(serde_json::Error),

    #[error("missing tail target")]
    MissingTailTarget,
    #[error("invalid tail target: {0}")]
    InvalidTailTarget(String),
    #[error("target with name {} not found", .0)]
    TailTargetNotFound(String),

    #[error("unknown path: {0}. must be one of: tail/object/<name>, tail/select/<query>")]
    UnknownPath(String),

    #[error("invalid datasource settings: {0}")]
    InvalidDatasourceSettings(#[from] serde_json::Error),

    #[error("Datasource not present on request")]
    MissingDatasource,

    #[error("Connection error: {0}")]
    Connection(#[from] tokio_postgres::Error),

    #[error("Error creating channel: {0}")]
    CreatingChannel(live::ChannelError),

    #[error("Error converting data: {0}")]
    ConvertTo(#[from] backend::ConvertToError),
    #[error("Error converting request: {0}")]
    ConvertFrom(#[from] backend::ConvertFromError),
    #[error("Error creating frame : {0}")]
    Data(#[from] data::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
