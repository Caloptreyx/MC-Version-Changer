//! Detects what a server currently runs: its jar is looked up on mcjars by SHA-256, the
//! `.mcvc-type.json` marker (written by this extension and by MC Version Chooser) is the fallback.

use crate::{
    mcjars::{self, LatestBuild},
    settings::ExtensionSettingsData,
};
use serde::{Deserialize, Serialize};
use shared::{
    State,
    models::{server::Server, server_variable::ServerVariable},
};
use tokio::io::AsyncReadExt;
use utoipa::ToSchema;
use wings_api::client::WingsClient;

/// Marker file shared with MC Version Chooser, read by the Modpack and Mod Installer extensions.
pub const MARKER_FILE: &str = ".mcvc-type.json";
const MAX_MARKER_SIZE: u64 = 64 * 1024;
pub const DEFAULT_JARFILE: &str = "server.jar";

#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DetectionSource {
    /// The server jar matched a build on mcjars.
    Jar,
    /// Read from `.mcvc-type.json`.
    Marker,
}

#[derive(ToSchema, Serialize)]
pub struct CurrentVersion {
    /// mcjars type id, e.g. `PAPER`.
    pub server_type: String,
    /// Minecraft version, or the project version for proxies and limbos.
    pub version: Option<String>,
    pub build_id: Option<i32>,
    pub build_name: Option<String>,
    pub source: DetectionSource,
    pub installed_at: Option<String>,
    /// The newest build of the same version, when the server runs an older one.
    #[schema(inline)]
    pub update: Option<LatestBuild>,
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

async fn read_marker(client: &WingsClient, server: uuid::Uuid) -> Option<Marker> {
    let reader = client
        .get_servers_server_files_contents(
            server,
            &wings_api::servers_server_files_contents::get::Query {
                file: Some(MARKER_FILE.into()),
                max_size: Some(MAX_MARKER_SIZE),
                ..Default::default()
            },
        )
        .await
        .ok()?;

    let mut content = Vec::new();
    reader
        .take(MAX_MARKER_SIZE)
        .read_to_end(&mut content)
        .await
        .ok()?;
    serde_json::from_slice(&content).ok()
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

/// Detects the server type, version and build. Failures to reach Wings or mcjars only
/// reduce what can be detected.
pub async fn detect(
    state: &State,
    settings: &ExtensionSettingsData,
    server: &Server,
    jar: &str,
) -> Option<CurrentVersion> {
    let node = server.node.fetch_cached(&state.database).await.ok()?;
    let client = node.api_client(&state.database).await.ok()?;

    let (hash, marker) = tokio::join!(jar_sha256(&client, server.uuid, jar), read_marker(&client, server.uuid));
    let marker = marker.unwrap_or_default();

    let lookup = match hash {
        Some(hash) => mcjars::lookup_jar(state, settings, &hash)
            .await
            .inspect_err(|err| tracing::debug!("mcjars jar lookup failed: {err:#}"))
            .ok()
            .flatten(),
        None => None,
    };

    if let Some(lookup) = lookup {
        let build = lookup.build;
        let installed_at = marker
            .installed_at
            .filter(|_| marker.build_id == Some(build.id));

        return Some(CurrentVersion {
            version: build.version().map(str::to_string),
            update: (lookup.latest.id != build.id && lookup.latest.installable()).then(|| LatestBuild {
                id: lookup.latest.id,
                name: lookup.latest.name,
                experimental: lookup.latest.experimental,
            }),
            server_type: build.server_type,
            build_id: Some(build.id),
            build_name: Some(build.name),
            source: DetectionSource::Jar,
            installed_at,
        });
    }

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

#[cfg(test)]
mod tests {
    use super::Marker;

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
}
