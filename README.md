# Grafana Materialize Data Source

This is a WIP Grafana data source which can connect to the [Materialize][] streaming SQL database.

## Screenshots

https://user-images.githubusercontent.com/5464991/166680691-8df200d7-e354-43bf-a924-8ce9fbc8582a.mov

![image](https://raw.githubusercontent.com/sd2k/grafana-materialize-datasource/main/src/img/query.png)

![image](https://raw.githubusercontent.com/sd2k/grafana-materialize-datasource/main/src/img/transforms.png)

## Getting started

### Using docker-compose

Running `docker-compose up -d` from the repository root will start up a Grafana instance with a
pre-provisioned instance of this data source pointing to a materialized instance inside Docker. It
will also start up two processes to watch the backend and frontend directory for changes,
rebuild the plugin components, and restart the backend process inside the Grafana container.

### Outside of Docker

#### Plugin frontend

At the repository root:

1. Install dependencies

   ```bash
   yarn install
   ```

2. Build plugin in development mode or run in watch mode

   ```bash
   yarn dev
   ```

   or

   ```bash
   yarn watch
   ```

3. Build plugin in production mode

   ```bash
   yarn build
   ```

#### Plugin backend

Make sure you have a recent version of Rust (run `rustup update stable`), and install [`cargo-watch`].

Then run:

```bash
cargo xtask watch
```

This will run the `watch` task using the [`cargo-xtask`] pattern, which rebuilds the backend component on changes, copies the binary into the correct location, and restarts the plugin process (which Grafana subsequently restarts).

#### Running Grafana

You can run a Grafana instance either by cloning the Grafana repository, or running it inside Docker. See the Grafana docs for more information.

## Cross compiling

### From MacOS

1. Install the relevant cross compiler toolchains. Using Homebrew:

   ```bash
   brew tap messense/macos-cross-toolchains
   brew install armv7-unknown-linux-musleabihf
   brew install aarch64-unknown-linux-musl
   brew install x86_64-unknown-linux-musl
   brew install mingw-w64
   ```

2. Install the relevant Rust targets. Using `rustup`:

   ```bash
   rustup target add armv7-unknown-linux-musleabihf
   rustup target add aarch64-apple-darwin
   rustup target add x86_64-apple-darwin
   rustup target add aarch64-unknown-linux-musl
   rustup target add x86_64-unknown-linux-musl
   rustup target add x86_64-pc-windows-gnu
   ```

3. Run the following to compile the plugin in release mode for each target:

   ```bash
   CARGO_TARGET_ARMV7_UNKNOWN_LINUX_MUSLEABIHF_LINKER=armv7-unknown-linux-musleabihf-ld cargo build --release --target armv7-unknown-linux-musleabihf --bin grafana-materialize-datasource
   cargo build --release --target aarch64-apple-darwin --bin grafana-materialize-datasource
   cargo build --release --target x86_64-apple-darwin --bin grafana-materialize-datasource
   CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=x86_64-unknown-linux-musl-gcc cargo build --release --target x86_64-unknown-linux-musl --bin grafana-materialize-datasource
   CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-unknown-linux-musl-gcc cargo build --release --target aarch64-unknown-linux-musl --bin grafana-materialize-datasource
   CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc cargo build --release --target x86_64-pc-windows-gnu --bin grafana-materialize-datasource
   ```

[`cargo-xtask`]: https://github.com/matklad/cargo-xtask
[`cargo-watch`]: https://github.com/watchexec/cargo-watch/
[Materialize]: https://materialize.com

