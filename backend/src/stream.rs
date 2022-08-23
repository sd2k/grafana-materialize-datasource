/// The `grafana_plugin_sdk::backend::StreamService` implementation for the Materialize plugin.
use futures_util::TryStreamExt;
use grafana_plugin_sdk::{backend, data};
use tracing::debug;

use crate::{queries::Query, rows_to_frame, Error, MaterializePlugin, Result};

/// Convert a Grafana Plugin SDK Frame to some initial data to send to new subscribers.
fn frame_to_initial_data(frame: &data::Frame) -> Result<backend::InitialData> {
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
        let query = Query::try_from_path(&request.path, self.sql_queries.clone()).await?;
        let target = query.as_tail()?;
        let datasource_settings = request
            .plugin_context
            .datasource_instance_settings
            .ok_or(Error::MissingDatasource)?;
        let client = self.get_client(&datasource_settings).await?;

        let initial_rows = target.select_all(&client).await?;

        Ok(backend::SubscribeStreamResponse::ok(Some(
            frame_to_initial_data(&rows_to_frame(&initial_rows))?,
        )))
    }

    type Error = Error;
    type Stream = backend::BoxRunStream<Self::Error>;

    /// Begin streaming data for a given channel.
    ///
    /// This method is called _once_ for a (datasource, path) combination and the output
    /// is multiplexed to all clients by Grafana's backend. This is in contrast to the
    /// `subscribe_stream` method which is called for every client that wishes to connect.
    async fn run_stream(&self, request: backend::RunStreamRequest) -> Result<Self::Stream> {
        let query = Query::try_from_path(&request.path, self.sql_queries.clone()).await?;
        let target = query.as_tail()?;
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
                    rows_to_frame(&[row])
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
