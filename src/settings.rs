use shared::extensions::settings::{
    ExtensionSettings, SettingsDeserializeExt, SettingsDeserializer, SettingsSerializeExt,
    SettingsSerializer,
};

pub const DEFAULT_API_URL: &str = "https://mcjars.app";
pub const DEFAULT_INSTALLER_IMAGE: &str = "python:3.13-slim";

#[derive(Clone)]
pub struct ExtensionSettingsData {
    /// Base URL of the mcjars instance (`<url>/api/v2/...`), without a trailing slash.
    pub api_url: String,
    /// Docker image the installation script runs in. Needs `python3` (3.10+).
    pub installer_image: String,
    /// Whether users may wipe all server files when changing the version.
    pub allow_clean_install: bool,
}

impl Default for ExtensionSettingsData {
    fn default() -> Self {
        Self {
            api_url: DEFAULT_API_URL.to_string(),
            installer_image: DEFAULT_INSTALLER_IMAGE.to_string(),
            allow_clean_install: true,
        }
    }
}

#[async_trait::async_trait]
impl SettingsSerializeExt for ExtensionSettingsData {
    async fn serialize(
        &self,
        serializer: SettingsSerializer,
    ) -> Result<SettingsSerializer, anyhow::Error> {
        Ok(serializer
            .write_raw_setting("api_url", &*self.api_url)
            .write_raw_setting("installer_image", &*self.installer_image)
            .write_serde_setting("allow_clean_install", &self.allow_clean_install)?)
    }
}

pub struct ExtensionSettingsDataDeserializer;

#[async_trait::async_trait]
impl SettingsDeserializeExt for ExtensionSettingsDataDeserializer {
    async fn deserialize_boxed(
        &self,
        deserializer: SettingsDeserializer<'_>,
    ) -> Result<ExtensionSettings, anyhow::Error> {
        let defaults = ExtensionSettingsData::default();

        Ok(Box::new(ExtensionSettingsData {
            api_url: deserializer
                .read_raw_setting("api_url")
                .filter(|url| !url.is_empty())
                .map(|url| url.to_string())
                .unwrap_or(defaults.api_url),
            installer_image: deserializer
                .read_raw_setting("installer_image")
                .filter(|image| !image.is_empty())
                .map(|image| image.to_string())
                .unwrap_or(defaults.installer_image),
            allow_clean_install: deserializer
                .read_serde_setting("allow_clean_install")
                .unwrap_or(defaults.allow_clean_install),
        }))
    }
}

/// Loads a copy of the extension settings.
pub async fn load(state: &shared::State) -> Result<ExtensionSettingsData, anyhow::Error> {
    Ok(state
        .settings
        .get()
        .await?
        .find_extension_settings::<ExtensionSettingsData>()?
        .clone())
}

/// Normalizes an administrator-entered mcjars URL: `http(s)://host[/path]` without a trailing
/// slash or a trailing `/api`. Returns `None` for anything else.
pub fn normalize_api_url(value: &str) -> Option<String> {
    let parsed = url::Url::parse(value.trim()).ok()?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || !parsed.username().is_empty()
    {
        return None;
    }

    let url = parsed.as_str().trim_end_matches('/');
    Some(url.strip_suffix("/api").unwrap_or(url).to_string())
}

#[cfg(test)]
mod tests {
    use super::normalize_api_url;

    #[test]
    fn api_urls_are_normalized() {
        assert_eq!(normalize_api_url("https://mcjars.app").as_deref(), Some("https://mcjars.app"));
        assert_eq!(normalize_api_url(" https://mcjars.app/api/ ").as_deref(), Some("https://mcjars.app"));
        assert_eq!(
            normalize_api_url("http://10.0.0.5:8000/mirror/").as_deref(),
            Some("http://10.0.0.5:8000/mirror")
        );
        assert_eq!(normalize_api_url("ftp://mcjars.app"), None);
        assert_eq!(normalize_api_url("mcjars.app"), None);
        assert_eq!(normalize_api_url("https://mcjars.app/?x=1"), None);
        assert_eq!(normalize_api_url("https://user@mcjars.app"), None);
    }
}
