//! Client for the mcjars API (<https://mcjars.app/api>): server types, their versions and
//! builds, and the reverse lookup of a jar by its hash. Responses are normalized into the
//! types below and cached through `state.cache`.

use crate::settings::ExtensionSettingsData;
use axum::http::StatusCode;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use shared::{State, response::DisplayError};
use std::{sync::LazyLock, time::Duration};
use utoipa::ToSchema;

/// Seconds the server type list is cached for.
const TYPES_TTL: u64 = 60 * 60;
/// Seconds version and build lists are cached for.
const LIST_TTL: u64 = 5 * 60;
/// Seconds a jar hash lookup is cached for.
const LOOKUP_TTL: u64 = 10 * 60;

pub const USER_AGENT: &str = concat!(
    "Caloptreyx/MC-Version-Changer/",
    env!("CARGO_PKG_VERSION"),
    " (Calagopus extension dev.caloptreyx.versionchanger)"
);

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .expect("failed to build mcjars http client")
});

pub fn user_error(message: impl Into<String>, status: StatusCode) -> anyhow::Error {
    DisplayError::new(message.into()).with_status(status).into()
}

/// A server type id as used by mcjars (`PAPER`, `VELOCITY_CTD`, `LEGACYFABRIC`).
pub fn valid_type(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

/// A Minecraft or project version as used by mcjars (`1.21.1`, `26.3-rc-3`, `4.2.1-SNAPSHOT`).
pub fn valid_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'+'))
}

/// Validates an mcjars-provided path relative to the server root.
pub fn safe_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', '\0', ':'])
        && path
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

// ---------------------------------------------------------------------------------------------
// Normalized types, sent to clients and stored in the cache (no internally tagged enums or
// skipped fields: the cache serializes with MessagePack).
// ---------------------------------------------------------------------------------------------

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct ServerType {
    /// mcjars type id, e.g. `PAPER`.
    pub id: String,
    pub name: String,
    /// mcjars grouping: `recommended`, `established`, `experimental`, `miscellaneous` or `limbos`.
    pub group: String,
    pub icon: String,
    pub color: String,
    pub homepage: String,
    pub description: String,
    /// `plugins`, `modded`, `proxy`, `limbo`.
    pub categories: Vec<String>,
    pub deprecated: bool,
    pub experimental: bool,
    pub builds: u64,
    pub minecraft_versions: u64,
    pub project_versions: u64,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct LatestBuild {
    pub id: i32,
    pub name: String,
    pub experimental: bool,
}

#[derive(ToSchema, Serialize, Deserialize, Clone)]
pub struct VersionSummary {
    /// Minecraft version, or the project version for types without one (proxies, limbos).
    pub id: String,
    pub snapshot: bool,
    pub supported: bool,
    /// Java feature release the version needs.
    pub java: u32,
    pub builds: u64,
    pub created: Option<String>,
    #[schema(inline)]
    pub latest: LatestBuild,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum StepAction {
    Download,
    Unzip,
    Remove,
}

/// One mcjars installation step. `download` uses `url`, `file` and `size`; `unzip` uses `file`
/// and `location`; `remove` uses `location`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Step {
    pub action: StepAction,
    pub url: Option<String>,
    pub file: Option<String>,
    pub location: Option<String>,
    pub size: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Build {
    pub id: i32,
    pub server_type: String,
    pub name: String,
    pub build_number: i32,
    pub experimental: bool,
    pub minecraft_version: Option<String>,
    pub project_version: Option<String>,
    pub created: Option<String>,
    pub size: Option<u64>,
    pub changes: Vec<String>,
    /// Steps, grouped as returned by mcjars. Empty when the build is not installable
    /// (no steps, an unsupported step or an unsafe path).
    pub installation: Vec<Vec<Step>>,
}

impl Build {
    pub fn installable(&self) -> bool {
        !self.installation.is_empty()
    }

    /// Whether the build is installed from an archive (Forge, NeoForge, ...).
    pub fn archive(&self) -> bool {
        self.installation
            .iter()
            .flatten()
            .any(|step| step.action == StepAction::Unzip)
    }

    /// The version the build is listed under: the Minecraft version, else the project version.
    pub fn version(&self) -> Option<&str> {
        self.minecraft_version
            .as_deref()
            .or(self.project_version.as_deref())
    }

    pub fn summary(&self) -> BuildSummary {
        BuildSummary {
            id: self.id,
            name: self.name.clone(),
            build_number: self.build_number,
            experimental: self.experimental,
            minecraft_version: self.minecraft_version.clone(),
            project_version: self.project_version.clone(),
            created: self.created.clone(),
            size: self.size,
            archive: self.archive(),
            installable: self.installable(),
            changes: self.changes.clone(),
        }
    }
}

#[derive(ToSchema, Serialize, Clone)]
pub struct BuildSummary {
    pub id: i32,
    pub name: String,
    pub build_number: i32,
    pub experimental: bool,
    pub minecraft_version: Option<String>,
    pub project_version: Option<String>,
    pub created: Option<String>,
    /// Download size in bytes.
    pub size: Option<u64>,
    /// Installed from an archive that replaces `libraries/`.
    pub archive: bool,
    pub installable: bool,
    pub changes: Vec<String>,
}

/// Result of looking a jar up by its hash.
#[derive(Serialize, Deserialize, Clone)]
pub struct JarLookup {
    pub build: Build,
    pub latest: Build,
}

// ---------------------------------------------------------------------------------------------
// Raw mcjars responses
// ---------------------------------------------------------------------------------------------

#[derive(Deserialize)]
struct RawTypes {
    types: IndexMap<String, IndexMap<String, RawTypeInfo>>,
}

#[derive(Deserialize)]
struct RawTypeVersions {
    #[serde(default)]
    minecraft: u64,
    #[serde(default)]
    project: u64,
}

#[derive(Deserialize)]
struct RawTypeInfo {
    name: String,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    color: String,
    #[serde(default)]
    homepage: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    deprecated: bool,
    #[serde(default)]
    experimental: bool,
    #[serde(default)]
    builds: u64,
    versions: RawTypeVersions,
}

#[derive(Deserialize)]
struct RawVersions {
    builds: IndexMap<String, RawVersion>,
}

#[derive(Deserialize)]
struct RawVersion {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    supported: bool,
    #[serde(default)]
    java: u32,
    #[serde(default)]
    builds: u64,
    created: Option<String>,
    latest: RawBuild,
}

#[derive(Deserialize)]
struct RawBuilds {
    builds: Vec<RawBuild>,
}

#[derive(Deserialize)]
struct RawLookup {
    build: RawBuild,
    latest: RawBuild,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RawStep {
    Download { url: String, file: String, size: u64 },
    Unzip { file: String, location: String },
    Remove { location: String },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBuild {
    id: i32,
    #[serde(rename = "type")]
    server_type: String,
    name: String,
    #[serde(default)]
    build_number: i32,
    #[serde(default)]
    experimental: bool,
    version_id: Option<String>,
    project_version_id: Option<String>,
    created: Option<String>,
    jar_size: Option<u64>,
    zip_size: Option<u64>,
    #[serde(default)]
    changes: Vec<String>,
    #[serde(default)]
    installation: Vec<Vec<serde_json::Value>>,
}

fn normalize_step(raw: serde_json::Value) -> Option<Step> {
    let step = match serde_json::from_value::<RawStep>(raw).ok()? {
        RawStep::Download { url, file, size } => {
            let parsed = url::Url::parse(&url).ok()?;
            if !matches!(parsed.scheme(), "http" | "https") || !safe_relative(&file) {
                return None;
            }
            Step {
                action: StepAction::Download,
                url: Some(url),
                file: Some(file),
                location: None,
                size: Some(size),
            }
        }
        RawStep::Unzip { file, location } => {
            let location = location.trim_start_matches("./").trim_end_matches('/');
            let location = if location == "." { "" } else { location };
            if !safe_relative(&file) || !(location.is_empty() || safe_relative(location)) {
                return None;
            }
            Step {
                action: StepAction::Unzip,
                url: None,
                file: Some(file),
                location: Some(location.to_string()),
                size: None,
            }
        }
        RawStep::Remove { location } => {
            if !safe_relative(&location) {
                return None;
            }
            Step {
                action: StepAction::Remove,
                url: None,
                file: None,
                location: Some(location),
                size: None,
            }
        }
    };

    Some(step)
}

impl From<RawBuild> for Build {
    fn from(raw: RawBuild) -> Self {
        // One unusable step makes the whole build unusable rather than half-installed.
        let installation = raw
            .installation
            .into_iter()
            .map(|group| group.into_iter().map(normalize_step).collect::<Option<Vec<_>>>())
            .collect::<Option<Vec<_>>>()
            .filter(|groups| groups.iter().flatten().any(|step| step.action == StepAction::Download))
            .unwrap_or_default();

        Self {
            id: raw.id,
            server_type: raw.server_type,
            name: raw.name,
            build_number: raw.build_number,
            experimental: raw.experimental,
            minecraft_version: raw.version_id,
            project_version: raw.project_version_id,
            created: raw.created,
            size: raw.zip_size.or(raw.jar_size),
            changes: raw.changes,
            installation,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------------------------

#[derive(Deserialize)]
struct RawError {
    #[serde(default)]
    errors: Vec<String>,
}

fn endpoint(settings: &ExtensionSettingsData, segments: &[&str]) -> Result<url::Url, anyhow::Error> {
    let mut url = url::Url::parse(&settings.api_url)
        .map_err(|_| user_error("the configured mcjars URL is invalid", StatusCode::BAD_GATEWAY))?;
    url.path_segments_mut()
        .map_err(|_| user_error("the configured mcjars URL is invalid", StatusCode::BAD_GATEWAY))?
        .pop_if_empty()
        .push("api")
        .extend(segments);

    Ok(url)
}

/// Sends a request to mcjars and decodes its JSON body. `Ok(None)` on 404.
async fn send<T: DeserializeOwned>(request: reqwest::RequestBuilder) -> Result<Option<T>, anyhow::Error> {
    let response = request
        .send()
        .await
        .map_err(|err| user_error(format!("mcjars is unreachable: {err}"), StatusCode::BAD_GATEWAY))?;

    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !status.is_success() {
        let detail = response
            .json::<RawError>()
            .await
            .ok()
            .and_then(|body| body.errors.into_iter().next())
            .map(|message| format!(": {message}"))
            .unwrap_or_default();
        return Err(user_error(
            format!("mcjars returned HTTP {status}{detail}"),
            StatusCode::BAD_GATEWAY,
        ));
    }

    response.json::<T>().await.map(Some).map_err(|err| {
        user_error(
            format!("unexpected response from mcjars: {err}"),
            StatusCode::BAD_GATEWAY,
        )
    })
}

async fn get<T: DeserializeOwned>(
    settings: &ExtensionSettingsData,
    segments: &[&str],
    not_found: &str,
) -> Result<T, anyhow::Error> {
    send(CLIENT.get(endpoint(settings, segments)?))
        .await?
        .ok_or_else(|| user_error(not_found, StatusCode::NOT_FOUND))
}

/// Every server type mcjars offers, in mcjars' order (recommended first).
pub async fn types(state: &State, settings: &ExtensionSettingsData) -> Result<Vec<ServerType>, anyhow::Error> {
    let key = format!("versionchanger::{}::types", settings.api_url);
    state
        .cache
        .cached(&key, TYPES_TTL, || async move {
            let raw: RawTypes = get(settings, &["v2", "types"], "mcjars has no server types").await?;

            Ok::<_, anyhow::Error>(
                raw.types
                    .into_iter()
                    .flat_map(|(group, types)| {
                        types.into_iter().map(move |(id, info)| ServerType {
                            id,
                            name: info.name,
                            group: group.clone(),
                            icon: info.icon,
                            color: info.color,
                            homepage: info.homepage,
                            description: info.description,
                            categories: info.categories,
                            deprecated: info.deprecated,
                            experimental: info.experimental,
                            builds: info.builds,
                            minecraft_versions: info.versions.minecraft,
                            project_versions: info.versions.project,
                        })
                    })
                    .collect(),
            )
        })
        .await
}

/// The versions of a server type, newest first.
pub async fn versions(
    state: &State,
    settings: &ExtensionSettingsData,
    server_type: &str,
) -> Result<Vec<VersionSummary>, anyhow::Error> {
    if !valid_type(server_type) {
        return Err(user_error("unknown server type", StatusCode::NOT_FOUND));
    }

    let key = format!("versionchanger::{}::versions::{server_type}", settings.api_url);
    state
        .cache
        .cached(&key, LIST_TTL, || async move {
            let raw: RawVersions =
                get(settings, &["v2", "builds", server_type], "unknown server type").await?;

            Ok::<_, anyhow::Error>(
                raw.builds
                    .into_iter()
                    .rev()
                    .map(|(id, version)| VersionSummary {
                        id,
                        snapshot: !version.kind.eq_ignore_ascii_case("release"),
                        supported: version.supported,
                        java: version.java,
                        builds: version.builds,
                        created: version.created,
                        latest: LatestBuild {
                            id: version.latest.id,
                            name: version.latest.name,
                            experimental: version.latest.experimental,
                        },
                    })
                    .collect(),
            )
        })
        .await
}

/// The builds of one version of a server type, newest first.
pub async fn builds(
    state: &State,
    settings: &ExtensionSettingsData,
    server_type: &str,
    version: &str,
) -> Result<Vec<Build>, anyhow::Error> {
    if !valid_type(server_type) || !valid_version(version) {
        return Err(user_error("unknown server type or version", StatusCode::NOT_FOUND));
    }

    let key = format!("versionchanger::{}::builds::{server_type}::{version}", settings.api_url);
    state
        .cache
        .cached(&key, LIST_TTL, || async move {
            let raw: RawBuilds = get(
                settings,
                &["v2", "builds", server_type, version],
                "unknown server type or version",
            )
            .await?;

            Ok::<_, anyhow::Error>(raw.builds.into_iter().map(Build::from).collect::<Vec<_>>())
        })
        .await
}

/// Finds the mcjars build a jar belongs to by its SHA-256. `None` when mcjars does not know it.
pub async fn lookup_jar(
    state: &State,
    settings: &ExtensionSettingsData,
    sha256: &str,
) -> Result<Option<JarLookup>, anyhow::Error> {
    let key = format!("versionchanger::{}::lookup::{sha256}", settings.api_url);
    state
        .cache
        .cached(&key, LOOKUP_TTL, || async move {
            let raw: Option<RawLookup> = send(
                CLIENT
                    .post(endpoint(settings, &["v2", "build"])?)
                    .json(&serde_json::json!({ "hash": { "sha256": sha256 } })),
            )
            .await?;

            Ok::<_, anyhow::Error>(raw.map(|raw| JarLookup {
                build: raw.build.into(),
                latest: raw.latest.into(),
            }))
        })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_must_stay_inside_the_server() {
        assert!(safe_relative("server.jar"));
        assert!(safe_relative("libraries/net/forge.jar"));
        for path in ["", "/etc/passwd", "../x", "a/../../x", "./x", "C:\\x", "a\\b", "a:b", "a/./b"] {
            assert!(!safe_relative(path), "{path}");
        }
    }

    #[test]
    fn type_and_version_ids_are_restricted() {
        assert!(valid_type("PAPER") && valid_type("VELOCITY_CTD") && valid_type("LEGACYFABRIC"));
        assert!(!valid_type("paper") && !valid_type("../x") && !valid_type(""));
        assert!(valid_version("1.21.1") && valid_version("26.3-rc-3") && valid_version("4.2.1-SNAPSHOT"));
        assert!(!valid_version("1.21/../x") && !valid_version("") && !valid_version("a b"));
    }

    fn raw_build(installation: serde_json::Value) -> RawBuild {
        serde_json::from_value(serde_json::json!({
            "id": 289498, "uuid": "9530882a-0241-483b-894a-57c0097a5ff2", "versionId": "1.20.1",
            "projectVersionId": "47.4.23", "type": "FORGE", "experimental": false, "name": "47.4.23",
            "buildNumber": 1, "jarUrl": null, "jarSize": null, "zipSize": 96053043,
            "zipUrl": "https://files.mcjars.app/forge/1.20.1/47.4.23/1/server.jar.zip",
            "installation": installation, "changes": [], "created": null,
        }))
        .unwrap()
    }

    #[test]
    fn archive_builds_keep_their_steps() {
        let build = Build::from(raw_build(serde_json::json!([
            [
                {"type": "download", "url": "https://files.mcjars.app/forge/1.20.1/47.4.23/1/server.jar.zip",
                 "file": "mcvapi.server.jar.zip", "size": 96053043},
                {"type": "remove", "location": "libraries"},
            ],
            [{"type": "unzip", "file": "mcvapi.server.jar.zip", "location": "."}],
            [{"type": "remove", "location": "mcvapi.server.jar.zip"}],
        ])));

        assert!(build.installable() && build.archive());
        assert_eq!(build.version(), Some("1.20.1"));
        assert_eq!(build.size, Some(96053043));
        assert_eq!(build.installation[1][0].location.as_deref(), Some(""));
    }

    #[test]
    fn builds_with_unsafe_or_unknown_steps_are_not_installable() {
        for installation in [
            serde_json::json!([[{"type": "download", "url": "https://x.test/a.jar", "file": "../a.jar", "size": 1}]]),
            serde_json::json!([[{"type": "download", "url": "file:///etc/passwd", "file": "a.jar", "size": 1}]]),
            serde_json::json!([[{"type": "download", "url": "https://x.test/a.jar", "file": "a.jar", "size": 1}],
                               [{"type": "exec", "command": "rm -rf /"}]]),
            serde_json::json!([[{"type": "remove", "location": "/"}]]),
            serde_json::json!([]),
        ] {
            assert!(!Build::from(raw_build(installation.clone())).installable(), "{installation}");
        }
    }

    #[test]
    fn cached_builds_round_trip_through_messagepack() {
        let build = Build::from(raw_build(serde_json::json!([[
            {"type": "download", "url": "https://x.test/a.jar", "file": "server.jar", "size": 5}
        ]])));
        let bytes = rmp_serde::to_vec(&vec![build]).unwrap();
        let decoded: Vec<Build> = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded[0].installation[0][0].action, StepAction::Download);
        assert_eq!(decoded[0].installation[0][0].size, Some(5));
    }
}
