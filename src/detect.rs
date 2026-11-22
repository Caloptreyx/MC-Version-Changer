//! Detects what a server currently runs. In order:
//! 1. the server jar, looked up on mcjars by SHA-256;
//! 2. the Forge/NeoForge launch arguments (`unix_args.txt`, or the one `run.sh` starts), looked up on
//!    mcjars by type and versions: these servers start a launcher stub jar that identifies nothing;
//! 3. the `.mcvc-type.json` marker written by this extension and by MC Version Chooser.

use crate::{
    mcjars::{self, BuildLookup, LatestBuild},
    settings::ExtensionSettingsData,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use shared::{
    State,
    models::{server::Server, server_variable::ServerVariable},
};
use std::sync::LazyLock;
use tokio::io::AsyncReadExt;
use utoipa::ToSchema;
use wings_api::client::WingsClient;

/// Marker file shared with MC Version Chooser, read by the Modpack and Mod Installer extensions.
pub const MARKER_FILE: &str = ".mcvc-type.json";
const MAX_MARKER_SIZE: u64 = 64 * 1024;
pub const DEFAULT_JARFILE: &str = "server.jar";
/// Files holding Forge/NeoForge launch arguments, in the order the eggs prefer them.
const ARGS_FILES: [&str; 2] = ["unix_args.txt", "run.sh"];
const MAX_ARGS_SIZE: u64 = 256 * 1024;

/// `forge-<minecraft>-<forge>-shim.jar`, which Forge 1.20.3+ starts instead of `--fml.*` options.
static FORGE_SHIM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^forge-(\d[\w.+]*)-(\d[\w.+-]*)-shim\.jar$").expect("valid regex"));

#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DetectionSource {
    /// The server jar matched a build on mcjars.
    Jar,
    /// Read from the Forge/NeoForge launch arguments.
    Args,
    /// Read from `.mcvc-type.json`.
    Marker,
}

#[derive(ToSchema, Serialize)]
pub struct CurrentVersion {
    /// mcjars type id, e.g. `PAPER`.
    pub server_type: String,
    /// Minecraft version, or the project version for proxies and limbos.
    pub version: Option<String>,
    /// The mcjars build, `null` when mcjars does not list it.
    pub build_id: Option<i32>,
    pub build_name: Option<String>,
    pub source: DetectionSource,
    pub installed_at: Option<String>,
    /// The newest build of the same version, when the server runs an older one.
    #[schema(inline)]
    pub update: Option<LatestBuild>,
}

#[derive(Default)]
pub struct Detection {
    pub current: Option<CurrentVersion>,
    /// mcjars could not be asked (unreachable or rate limited), so `current` may be missing or
    /// less precise than it would otherwise be.
    pub lookup_failed: bool,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Marker {
    #[serde(rename = "type")]
    server_type: Option<String>,
    version: Option<String>,
    build_id: Option<i32>,
    build_name: Option<String>,
    installed_at: Option<String>,
}

/// A Forge or NeoForge install as named by its launch arguments.
#[derive(Debug, PartialEq, Eq)]
struct LoaderArgs {
    server_type: &'static str,
    minecraft_version: String,
    loader_version: String,
}

/// The jar the egg starts (`SERVER_JARFILE`), `server.jar` when unset or unsafe.
pub async fn jar_file(state: &State, server: &Server) -> String {
    ServerVariable::all_by_server_uuid_egg_uuid(&state.database, server.uuid, server.egg.uuid)
        .await
        .ok()
        .and_then(|variables| {
            variables
                .into_iter()
                .find(|variable| variable.variable.env_variable.as_str() == "SERVER_JARFILE")
                .map(|variable| variable.value.trim().trim_start_matches("./").to_string())
        })
        .filter(|value| mcjars::safe_relative(value))
        .unwrap_or_else(|| DEFAULT_JARFILE.to_string())
}

/// Reads a file of at most `max_size` bytes. `None` when it is missing, larger or unreadable.
async fn read_file(client: &WingsClient, server: uuid::Uuid, file: &str, max_size: u64) -> Option<Vec<u8>> {
    let reader = client
        .get_servers_server_files_contents(
            server,
            &wings_api::servers_server_files_contents::get::Query {
                file: Some(file.into()),
                max_size: Some(max_size),
                ..Default::default()
            },
        )
        .await
        .ok()?;

    let mut content = Vec::new();
    reader.take(max_size).read_to_end(&mut content).await.ok()?;
    Some(content)
}

async fn read_marker(client: &WingsClient, server: uuid::Uuid) -> Option<Marker> {
    serde_json::from_slice(&read_file(client, server, MARKER_FILE, MAX_MARKER_SIZE).await?).ok()
}

async fn jar_sha256(client: &WingsClient, server: uuid::Uuid, jar: &str) -> Option<String> {
    client
        .get_servers_server_files_fingerprints(
            server,
            &wings_api::servers_server_files_fingerprints::get::Query {
                algorithm: Some(wings_api::Algorithm::Sha256),
                files: Some(vec![jar.into()]),
                ..Default::default()
            },
        )
        .await
        .ok()?
        .fingerprints
        .into_values()
        .map(|hash| hash.as_str().to_ascii_lowercase())
        .find(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

/// Reads the loader from Forge/NeoForge launch arguments: the `--fml.*` options of
/// `unix_args.txt`, or the shim jar Forge 1.20.3+ starts.
fn parse_loader_args(text: &str) -> Option<LoaderArgs> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let option = |name: &str| {
        tokens
            .windows(2)
            .find(|pair| pair[0] == name)
            .map(|pair| pair[1])
            .filter(|value| mcjars::valid_version(value))
    };

    if let Some(minecraft_version) = option("--fml.mcVersion") {
        let loader = match (option("--fml.neoForgeVersion"), option("--fml.forgeVersion")) {
            (Some(version), _) => Some(("NEOFORGE", version)),
            // NeoForge for 1.20.1 still launches as Forge, under its own group.
            (None, Some(version)) if option("--fml.forgeGroup") == Some("net.neoforged") => {
                Some(("NEOFORGE", version))
            }
            (None, Some(version)) => Some(("FORGE", version)),
            (None, None) => None,
        };
        if let Some((server_type, version)) = loader {
            return Some(LoaderArgs {
                server_type,
                minecraft_version: minecraft_version.to_string(),
                loader_version: version.to_string(),
            });
        }
    }

    tokens.iter().find_map(|token| {
        let captures = FORGE_SHIM.captures(token)?;
        Some(LoaderArgs {
            server_type: "FORGE",
            minecraft_version: captures[1].to_string(),
            loader_version: captures[2].to_string(),
        })
    })
}

/// The arguments file a start script passes (`@libraries/.../unix_args.txt` in Forge's `run.sh`).
fn referenced_args_file(text: &str) -> Option<&str> {
    text.split_whitespace()
        .filter_map(|token| token.trim_matches(['"', '\'']).strip_prefix('@'))
        .find(|path| {
            path.starts_with("libraries/") && path.ends_with("/unix_args.txt") && mcjars::safe_relative(path)
        })
}

async fn loader_args(client: &WingsClient, server: uuid::Uuid) -> Option<LoaderArgs> {
    for file in ARGS_FILES {
        let Some(content) = read_file(client, server, file, MAX_ARGS_SIZE).await else {
            continue;
        };
        let text = String::from_utf8_lossy(&content);
        if let Some(args) = parse_loader_args(&text) {
            return Some(args);
        }

        let Some(referenced) = referenced_args_file(&text) else {
            continue;
        };
        if let Some(content) = read_file(client, server, referenced, MAX_ARGS_SIZE).await
            && let Some(args) = parse_loader_args(&String::from_utf8_lossy(&content))
        {
            return Some(args);
        }
    }

    None
}

fn from_lookup(lookup: BuildLookup, source: DetectionSource, marker: &Marker) -> CurrentVersion {
    let build = lookup.build;

    CurrentVersion {
        version: build.version().map(str::to_string),
        update: (lookup.latest.id != build.id && lookup.latest.installable()).then(|| LatestBuild {
            id: lookup.latest.id,
            name: lookup.latest.name,
            experimental: lookup.latest.experimental,
        }),
        installed_at: marker
            .installed_at
            .clone()
            .filter(|_| marker.build_id == Some(build.id)),
        server_type: build.server_type,
        build_id: Some(build.id),
        build_name: Some(build.name),
        source,
    }
}

fn from_marker(marker: Marker) -> Option<CurrentVersion> {
    let server_type = marker
        .server_type
        .map(|value| value.trim().to_ascii_uppercase())
        .filter(|value| mcjars::valid_type(value))?;

    Some(CurrentVersion {
        server_type,
        version: marker.version.filter(|value| mcjars::valid_version(value)),
        build_id: marker.build_id,
        build_name: marker.build_name,
        source: DetectionSource::Marker,
        installed_at: marker.installed_at,
        update: None,
    })
}

/// Detects the server type, version and build. Failures to reach Wings or mcjars only
/// reduce what can be detected.
pub async fn detect(state: &State, settings: &ExtensionSettingsData, server: &Server, jar: &str) -> Detection {
    let client = match server.node.fetch_cached(&state.database).await {
        Ok(node) => node.api_client(&state.database).await.ok(),
        Err(_) => None,
    };
    let Some(client) = client else {
        return Detection::default();
    };

    let (hash, marker) = tokio::join!(jar_sha256(&client, server.uuid, jar), read_marker(&client, server.uuid));
    let marker = marker.unwrap_or_default();
    let mut lookup_failed = false;

    if let Some(hash) = hash {
        match mcjars::lookup_jar(state, settings, &hash).await {
            Ok(Some(lookup)) => {
                return Detection {
                    current: Some(from_lookup(lookup, DetectionSource::Jar, &marker)),
                    lookup_failed: false,
                };
            }
            Ok(None) => {}
            Err(err) => {
                tracing::warn!(server = %server.uuid, "mcjars jar lookup failed: {err:#}");
                lookup_failed = true;
            }
        }
    }

    if let Some(args) = loader_args(&client, server.uuid).await {
        let lookup = mcjars::lookup_build(
            state,
            settings,
            args.server_type,
            &args.minecraft_version,
            &args.loader_version,
        )
        .await;

        match lookup {
            Ok(Some(lookup)) => {
                return Detection {
                    current: Some(from_lookup(lookup, DetectionSource::Args, &marker)),
                    lookup_failed: false,
                };
            }
            Ok(None) => {}
            Err(err) => {
                tracing::warn!(server = %server.uuid, "mcjars build lookup failed: {err:#}");
                lookup_failed = true;
            }
        }

        // mcjars does not list the build (or could not be asked): show what the arguments name.
        return Detection {
            current: Some(CurrentVersion {
                server_type: args.server_type.to_string(),
                version: Some(args.minecraft_version),
                build_id: None,
                build_name: Some(args.loader_version),
                source: DetectionSource::Args,
                installed_at: None,
                update: None,
            }),
            lookup_failed,
        };
    }

    Detection {
        current: from_marker(marker),
        lookup_failed,
    }
}

#[cfg(test)]
mod tests {
    use super::{LoaderArgs, Marker, parse_loader_args, referenced_args_file};

    #[test]
    fn markers_of_both_extensions_parse() {
        let ours: Marker = serde_json::from_str(
            r##"{"type":"PAPER","version":"1.21.1","buildId":298600,"buildName":"#142",
            "installedAt":"2026-10-02T10:00:00+00:00","installer":"dev.caloptreyx.versionchanger"}"##,
        )
        .unwrap();
        assert_eq!(ours.server_type.as_deref(), Some("PAPER"));
        assert_eq!(ours.build_id, Some(298600));

        let chooser: Marker =
            serde_json::from_str(r#"{"type":"FABRIC","version":"1.21.1","installedAt":"2026-08-01T00:00:00.000Z"}"#)
                .unwrap();
        assert_eq!(chooser.version.as_deref(), Some("1.21.1"));
        assert_eq!(chooser.build_id, None);
    }

    fn loader(server_type: &'static str, minecraft_version: &str, loader_version: &str) -> Option<LoaderArgs> {
        Some(LoaderArgs {
            server_type,
            minecraft_version: minecraft_version.to_string(),
            loader_version: loader_version.to_string(),
        })
    }

    #[test]
    fn loaders_are_read_from_launch_arguments() {
        // Tails of real unix_args.txt files.
        let neoforge = "-p libraries/cpw/mods/bootstraplauncher/2.0.2/bootstraplauncher-2.0.2.jar\n\
            cpw.mods.bootstraplauncher.BootstrapLauncher\n--launchTarget forgeserver\n\
            --fml.neoForgeVersion 21.1.257\n--fml.fmlVersion 4.0.45\n--fml.mcVersion 1.21.1\n\
            --fml.neoFormVersion 20240808.144430\n";
        assert_eq!(parse_loader_args(neoforge), loader("NEOFORGE", "1.21.1", "21.1.257"));

        let forge = "cpw.mods.bootstraplauncher.BootstrapLauncher\n--launchTarget forgeserver\n\
            --fml.forgeVersion 47.3.25\n--fml.mcVersion 1.20.1\n--fml.forgeGroup net.minecraftforge\n\
            --fml.mcpVersion 20230612.114412\n";
        assert_eq!(parse_loader_args(forge), loader("FORGE", "1.20.1", "47.3.25"));

        let neoforge_1_20_1 = "--launchTarget forgeserver --fml.forgeVersion 47.1.106 --fml.mcVersion 1.20.1 \
            --fml.forgeGroup net.neoforged --fml.mcpVersion 20230612.114412";
        assert_eq!(parse_loader_args(neoforge_1_20_1), loader("NEOFORGE", "1.20.1", "47.1.106"));

        let shim = "-Djava.net.preferIPv6Addresses=system -jar forge-1.21.10-60.1.0-shim.jar\n";
        assert_eq!(parse_loader_args(shim), loader("FORGE", "1.21.10", "60.1.0"));

        assert_eq!(parse_loader_args("java -Xmx2G -jar server.jar nogui"), None);
        assert_eq!(parse_loader_args("--fml.forgeVersion ../x --fml.mcVersion 1.20.1"), None);
    }

    #[test]
    fn run_scripts_point_at_their_arguments_file() {
        let run_sh = "#!/usr/bin/env sh\n# Add custom JVM arguments to the user_jvm_args.txt\n\
            java @user_jvm_args.txt @libraries/net/minecraftforge/forge/1.20.1-47.4.2/unix_args.txt \"$@\"\n";
        assert_eq!(
            referenced_args_file(run_sh),
            Some("libraries/net/minecraftforge/forge/1.20.1-47.4.2/unix_args.txt")
        );
        assert_eq!(referenced_args_file("java @libraries/../../etc/unix_args.txt"), None);
        assert_eq!(referenced_args_file("java -jar server.jar"), None);
    }
}
