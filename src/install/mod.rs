//! Builds the Wings installation script that installs an mcjars build.

use crate::mcjars::{Build, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use utoipa::ToSchema;
use wings_api::InstallationScript;

/// The installer, executed with the `python3` of the installer image.
const INSTALLER: &str = include_str!("install.py");
const HEREDOC_DELIMITER: &str = "MVC_INSTALLER_PY_EOF";

#[derive(ToSchema, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InstallMode {
    /// Keep worlds, plugins, mods and settings; replace the server jar (and `libraries/`).
    Replace,
    /// Delete every server file before installing.
    Wipe,
}

pub struct ScriptOptions<'a> {
    pub build: &'a Build,
    pub type_name: &'a str,
    pub jar_file: &'a str,
    pub mode: InstallMode,
    pub accept_eula: bool,
    pub image: &'a str,
}

fn wrapper() -> String {
    format!(
        "#!/bin/bash\nset -e\ncat > /tmp/version-changer.py <<'{HEREDOC_DELIMITER}'\n{INSTALLER}\n{HEREDOC_DELIMITER}\nexec python3 /tmp/version-changer.py\n"
    )
}

/// The build as passed to `install.py` in `MVC_BUILD`.
fn build_payload(build: &Build, type_name: &str) -> String {
    serde_json::json!({
        "type": build.server_type,
        "typeName": type_name,
        "version": build.version(),
        "minecraftVersion": build.minecraft_version,
        "projectVersion": build.project_version,
        "buildId": build.id,
        "buildName": build.name,
        "installation": build.installation,
    })
    .to_string()
}

pub fn script(options: ScriptOptions<'_>) -> InstallationScript {
    let mut environment = indexmap::IndexMap::new();
    let mut set = |key: &str, value: String| {
        environment.insert(
            compact_str::CompactString::from(key),
            serde_json::Value::String(value),
        );
    };

    set("MVC_BUILD", build_payload(options.build, options.type_name));
    set("MVC_JARFILE", options.jar_file.to_string());
    set(
        "MVC_MODE",
        match options.mode {
            InstallMode::Replace => "replace",
            InstallMode::Wipe => "wipe",
        }
        .to_string(),
    );
    set("MVC_ACCEPT_EULA", if options.accept_eula { "1" } else { "0" }.to_string());
    set("MVC_USER_AGENT", USER_AGENT.to_string());

    InstallationScript {
        container_image: options.image.into(),
        entrypoint: "/bin/bash".into(),
        script: wrapper().into(),
        environment,
    }
}

#[derive(ToSchema, Serialize, Clone)]
pub struct DockerImage {
    pub name: String,
    pub image: String,
    /// Java feature release the image provides, parsed from its name or tag.
    pub java_version: Option<u32>,
}

/// Parses the Java version out of an egg docker image name or tag
/// (`Java 21`, `ghcr.io/ptero-eggs/yolks:java_21`, `eclipse-temurin:17-jre`).
pub fn java_version_of(value: &str) -> Option<u32> {
    static JAVA: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)(?:java|jdk|jre|temurin|openjdk|zulu|graalvm|corretto)\D{0,3}?(\d{1,2})(?:\D|$)")
            .expect("invalid java regex")
    });
    JAVA.captures(value)
        .and_then(|captures| captures[1].parse().ok())
        .filter(|version| (7..=40).contains(version))
}

pub fn docker_images<'a, I>(images: I) -> Vec<DockerImage>
where
    I: IntoIterator<Item = (&'a compact_str::CompactString, &'a compact_str::CompactString)>,
{
    images
        .into_iter()
        .map(|(name, image)| DockerImage {
            name: name.to_string(),
            image: image.to_string(),
            java_version: java_version_of(name).or_else(|| java_version_of(image)),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_cannot_terminate_the_heredoc_early() {
        assert!(!INSTALLER.lines().any(|line| line.trim() == HEREDOC_DELIMITER));
        assert!(wrapper().ends_with("exec python3 /tmp/version-changer.py\n"));
    }

    #[test]
    fn java_versions_are_parsed_from_image_names_and_tags() {
        assert_eq!(java_version_of("Java 21"), Some(21));
        assert_eq!(java_version_of("ghcr.io/ptero-eggs/yolks:java_8"), Some(8));
        assert_eq!(java_version_of("eclipse-temurin:17-jre"), Some(17));
        assert_eq!(java_version_of("ghcr.io/ptero-eggs/yolks:java_25"), Some(25));
        assert_eq!(java_version_of("ghcr.io/ptero-eggs/yolks:debian"), None);
    }
}
