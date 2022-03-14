use std::{
    env, fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};
use phf::{phf_map, Map};
use serde_json::Value;

/// Map of Go targets, as understood by Grafana, to Rust targets.
static RUST_TARGETS: Map<&'static str, &'static str> = phf_map! {
    "linux_amd64" => "x86_64-unknown-linux-musl",
    "linux_arm" => "armv7-unknown-linux-musleabihf",
    "linux_arm64" => "aarch64-unknown-linux-musl",
    "darwin_amd64" => "x86_64-apple-darwin",
    "darwin_arm64" => "aarch64-apple-darwin",
    "windows_amd64" => "x86_64-pc-windows-gnu",
};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let cmd = args.next();
    let root_dir = root_dir()?;
    let bin = grafana_plugin_bin(root_dir.clone())?;
    match cmd.as_deref() {
        Some("watch") => {
            let profile = if args.next().as_deref() == Some("release") {
                Profile::Release
            } else {
                Profile::Debug
            };
            watch(profile, &bin)?
        }
        Some("build") | None => {
            let targets = args.collect();
            build_targets(targets, &bin, root_dir)?;
        }
        _ => print_help(),
    };
    Ok(())
}

fn print_help() {
    eprintln!(
        "Tasks:
watch [release]      watch for changes, then compile plugin (optionally in release mode), replace in `dist` directory, and restart plugin process
build [target ...]   build the plugin in release mode for the given target (or all targets if not supplied), then copy into `dist` directory.
"
    )
}

fn root_dir() -> Result<PathBuf> {
    env::var("CARGO_MANIFEST_DIR")
        .ok()
        .and_then(|x| PathBuf::from(x).parent().map(|x| x.to_path_buf()))
        .context("this executable must be invoked using `cargo xtask`")
}

// Taken from Cargo:
// https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

fn grafana_plugin_bin(root_dir: PathBuf) -> Result<String> {
    let mut plugin_json = root_dir;
    plugin_json.push("src");
    plugin_json.push("plugin.json");
    let contents = fs::read(&plugin_json).context("could not find plugin.json")?;
    let executable = serde_json::from_slice::<Value>(&contents)?
        .get("executable")
        .context("could not find executable in plugin.json")?
        .as_str()
        .context("unexpected type for 'executable' field in plugin.json")?
        .to_string();
    Ok(normalize_path(&PathBuf::from(executable))
        .to_string_lossy()
        .to_string())
}

fn go_target() -> Result<String> {
    env::var("GOARCH").or_else(|_| {
        let go_output = Command::new("go")
            .arg("version")
            .output()
            .context("go must be installed to fetch target host and arch; alternatively set GOARCH env var to e.g. darwin_arm64 or linux_amd64")?;
        String::from_utf8(go_output.stdout)?.trim().split(' ').nth(3).map(|s| s.replace('/', "_")).context("unexpected output from `go version`")
    })
}

#[derive(Copy, Clone, Debug)]
enum Profile {
    Debug,
    Release,
}

fn watch(profile: Profile, bin: &str) -> Result<()> {
    let go_target = go_target()?;
    let (build_cmd, cargo_target) = if matches!(profile, Profile::Release) {
        ("build --release", "release")
    } else {
        ("build", "debug")
    };
    let root_dir = root_dir()?;
    // Grafana expects the final file to be named e.g. 'grafana-my-plugin_darwin_amd64'
    // See 'executable' at
    // https://grafana.com/docs/grafana/latest/developers/plugins/metadata/#properties.
    let dist_file = format!("{bin}_{go_target}");
    let dist_path = {
        let mut path = root_dir.clone();
        path.push("dist");
        path.push(&dist_file);
        path
    };
    let target_path = {
        let mut path = root_dir;
        path.push("target");
        path.push(cargo_target);
        path.push(bin);
        path
    };
    let shell_cmd = format!(
        "rm -rf {dist_path} && cp {target_path} {dist_path} && pkill -HUP {dist_file}",
        dist_path = dist_path.display(),
        target_path = target_path.display(),
    );
    let mut handle = Command::new("cargo")
        .arg("watch")
        .arg("-w")
        .arg("backend")
        .arg("-x")
        .arg("clippy")
        .arg("-x")
        .arg(build_cmd)
        .arg("-s")
        .arg(&shell_cmd)
        .arg("-c")
        .spawn()?;
    Ok(handle.wait().map(|_| ())?)
}

fn build_target(target: &str, bin: &str, mut root_dir: PathBuf) -> Result<PathBuf> {
    Command::new("cargo")
        .args(&["build", "--release", "--target", target, "--bin", bin])
        .status()?
        .success()
        .then(|| {
            root_dir.push("target");
            root_dir.push(target);
            root_dir.push("release");
            root_dir.push(bin);
            if target.contains("windows") {
                root_dir.set_extension("exe");
            }
            root_dir
        })
        .with_context(|| format!("error building target {target}"))
}

fn cp_target_bin(path: &Path, root_dir: PathBuf, gotarget: &str, bin: &str) -> Result<()> {
    if !path.exists() {
        bail!("file not found: {}", path.display());
    }
    let mut dest = root_dir;
    dest.push("dist");
    let extension = if gotarget.starts_with("windows") {
        ".exe"
    } else {
        ""
    };
    dest.push(format!("{bin}_{gotarget}{extension}"));

    fs::copy(path, &dest)?;
    Ok(())
}

fn build_targets(mut go_targets: Vec<String>, bin: &str, root_dir: PathBuf) -> Result<()> {
    if go_targets.is_empty() {
        go_targets = RUST_TARGETS.keys().map(|k| k.to_string()).collect();
    }
    let rust_targets: Vec<_> = go_targets
        .iter()
        .map(|target| RUST_TARGETS.get(target.as_str()))
        .collect();
    let go_rust_targets = go_targets.iter().zip(rust_targets.iter());
    let invalid: Vec<_> = go_rust_targets
        .clone()
        .filter_map(|(t, r)| r.is_none().then(|| t))
        .collect();
    if !invalid.is_empty() {
        bail!(
            "unknown targets: {}",
            invalid
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    for (go_target, rust_target) in go_rust_targets {
        eprintln!("Building for {go_target}");
        let path = build_target(rust_target.unwrap(), bin, root_dir.clone())?;
        cp_target_bin(&path, root_dir.clone(), go_target, bin)?;
    }
    Ok(())
}
