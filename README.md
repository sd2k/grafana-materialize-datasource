# Grafana Materialize Data Source

This is a Grafana data source which can connect to the [Materialize][] streaming SQL database. It supports the Grafana Live capabilities for streaming data and hence can be used to visualise data in materialized views 'live'.

## Usage

### Configuring the datasource

Only three parameters are available for the datasource:

- **Host** - the hostname of the Materialize database
- **Port** - the port on which to connect to the Materialize database
- **Username** - the username as which to connect to the Materialize database

### Querying the datasource

When querying the datasource in a new panel you have two options available to you:

- **Relation** - the query builder will populate a list of available relations in the Materialize database. Select one and the relation will be `TAIL`ed to the panel.
- **Select statement** - input a custom statement into the query field and the output of the statement will be `TAIL`ed to the panel.

### Configuring panels

The plugin includes the `mz_timestamp` and `mz_diff` columns in the streaming output, which may not be what you want to see. The simplest way to solve this is to use the [Transformations][] functionality of the panels. In the panel editor, click the **Transform** button and add any transformations you like. A good place to start is:

1. Organize fields
   Use this to hide the `mz_diff` field by clicking the 'eye' symbol next to the field name.
2. Prepare time series
   Select **Multi-frame time series** as the format.
3. Rename by regex
   Use this to remove any common unwanted prefix from the time series names. E.g. set **Match** to `avg (.*)` and **Replace** to `$1` to remove the prefix `avg `.

See the 'transforms' screenshot for an example.

## Screenshots

https://user-images.githubusercontent.com/5464991/166680691-8df200d7-e354-43bf-a924-8ce9fbc8582a.mov

![image](https://raw.githubusercontent.com/sd2k/grafana-materialize-datasource/main/src/img/query.png)

![image](https://raw.githubusercontent.com/sd2k/grafana-materialize-datasource/main/src/img/transforms.png)

[Materialize]: https://materialize.com
[Transformations]: https://grafana.com/docs/grafana/latest/panels/transform-data/transformation-functions/

