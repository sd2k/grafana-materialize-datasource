use grafana_materialize_datasource::MaterializePlugin;

#[grafana_plugin_sdk::main(
    services(data, diagnostics, resource, stream),
    init_subscriber = true,
    shutdown_handler = "0.0.0.0:10001"
)]
async fn plugin() -> MaterializePlugin {
    MaterializePlugin::default()
}
