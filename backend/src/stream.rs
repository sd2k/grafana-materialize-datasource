/// The `grafana_plugin_sdk::backend::StreamService` implementation for the Console plugin.
use futures_util::TryStreamExt;
use grafana_plugin_sdk::{backend, data};
use tracing::debug;

use crate::{path, rows_to_frame, Error, MaterializePlugin, Result};

/// Convert a Grafana Plugin SDK Frame to some initial data to send to new subscribers.
fn frame_to_initial_data(frame: data::Frame) -> Result<backend::InitialData> {
    let checked = frame.check()?;
    Ok(backend::InitialData::from_frame(
        checked,
        data::FrameInclude::All,
    )?)
}

#[backend::async_trait]
impl backend::StreamService for MaterializePlugin {
    type JsonValue = ();

    /// Subscribe to a stream of updates from a Materialize datasource instance.
    ///
    /// This function will be called every time a user subscribes to a stream.
    async fn subscribe_stream(
        &self,
        request: backend::SubscribeStreamRequest,
    ) -> Result<backend::SubscribeStreamResponse> {
        dbg!(&request);
        let path = request.path()?;
        let target = self.target(path).await?;
        let datasource_settings = request
            .plugin_context
            .datasource_instance_settings
            .ok_or(Error::MissingDatasource)?;
        let client = self.get_client(&datasource_settings).await?;

        let initial_rows = target.select_all(&client).await?;

        Ok(backend::SubscribeStreamResponse::new(
            backend::SubscribeStreamStatus::Ok,
            Some(frame_to_initial_data(rows_to_frame(initial_rows))?),
        ))
    }

    type Error = Error;
    type Stream = backend::BoxRunStream<Self::Error>;

    /// Begin streaming data for a given channel.
    ///
    /// This method is called _once_ for a (datasource, path) combination and the output
    /// is multiplexed to all clients by Grafana's backend. This is in contrast to the
    /// `subscribe_stream` method which is called for every client that wishes to connect.
    async fn run_stream(&self, request: backend::RunStreamRequest) -> Result<Self::Stream> {
        let path = request.path()?;
        let target = self.target(path).await?;
        let datasource_settings = request
            .plugin_context
            .datasource_instance_settings
            .ok_or(Error::MissingDatasource)?;
        let client = self.get_client(&datasource_settings).await?;

        let stream = Box::pin(
            target
                .tail(&client)
                .await?
                .map_err(Error::Connection)
                .and_then(|row| async {
                    rows_to_frame(vec![row])
                        .check()
                        .map_err(Error::Data)
                        .and_then(|f| Ok(backend::StreamPacket::from_frame(f)?))
                }),
        );

        Ok(stream)
    }

    async fn publish_stream(
        &self,
        _request: backend::PublishStreamRequest,
    ) -> Result<backend::PublishStreamResponse> {
        debug!("Publishing to stream is not implemented");
        unimplemented!()
    }
}

/// Extension trait providing some convenience methods for getting the `path` and `datasource_uid`.
trait StreamRequestExt {
    /// The path passed as part of the request, as a `&str`.
    fn raw_path(&self) -> &str;
    /// The datasource instance settings passed in the request.
    fn datasource_instance_settings(&self) -> Option<&backend::DataSourceInstanceSettings>;

    /// The parsed `Path`, or an `Error` if parsing failed.
    fn path(&self) -> Result<path::Path> {
        let path = self.raw_path();
        path.parse()
            .map_err(|_| Error::UnknownPath(path.to_string()))
    }
}

macro_rules! impl_stream_request_ext {
    ($request: path) => {
        impl StreamRequestExt for $request {
            fn raw_path(&self) -> &str {
                self.path.as_str()
            }

            fn datasource_instance_settings(&self) -> Option<&backend::DataSourceInstanceSettings> {
                self.plugin_context.datasource_instance_settings.as_ref()
            }
        }
    };
}

impl_stream_request_ext!(backend::RunStreamRequest);
impl_stream_request_ext!(backend::SubscribeStreamRequest);
impl_stream_request_ext!(backend::PublishStreamRequest);
