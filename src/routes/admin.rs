use super::State;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

#[derive(ToSchema, Serialize)]
pub struct AdminSettings {
    api_url: String,
    installer_image: String,
    allow_clean_install: bool,
}

impl From<&crate::settings::ExtensionSettingsData> for AdminSettings {
    fn from(settings: &crate::settings::ExtensionSettingsData) -> Self {
        Self {
            api_url: settings.api_url.clone(),
            installer_image: settings.installer_image.clone(),
            allow_clean_install: settings.allow_clean_install,
        }
    }
}

mod get {
    use super::AdminSettings;
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::user::GetPermissionManager,
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        #[schema(inline)]
        settings: AdminSettings,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
    ))]
    pub async fn route(state: GetState, permissions: GetPermissionManager) -> ApiResponseResult {
        permissions.has_admin_permission("version-changer.read")?;

        let settings = crate::settings::load(&state).await?;

        ApiResponse::new_serialized(Response {
            settings: AdminSettings::from(&settings),
        })
        .ok()
    }
}

mod put {
    use super::AdminSettings;
    use crate::settings::{ExtensionSettingsData, normalize_api_url};
    use axum::http::StatusCode;
    use garde::Validate;
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::{admin_activity::GetAdminActivityLogger, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Validate, Deserialize)]
    pub struct Payload {
        #[garde(length(chars, min = 1, max = 255))]
        #[schema(min_length = 1, max_length = 255)]
        api_url: String,
        #[garde(length(chars, min = 1, max = 255))]
        #[schema(min_length = 1, max_length = 255)]
        installer_image: String,
        #[garde(skip)]
        allow_clean_install: bool,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        #[schema(inline)]
        settings: AdminSettings,
    }

    #[utoipa::path(put, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        activity_logger: GetAdminActivityLogger,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        if let Err(errors) = shared::utils::validate_data(&data) {
            return ApiResponse::new_serialized(ApiError::new_strings_value(errors))
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        permissions.has_admin_permission("version-changer.manage")?;

        let Some(api_url) = normalize_api_url(&data.api_url) else {
            return ApiResponse::error("the mcjars URL must be an http(s) URL such as https://mcjars.app")
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        };
        let installer_image = data.installer_image.trim().to_string();
        if installer_image.is_empty() || installer_image.chars().any(char::is_whitespace) {
            return ApiResponse::error("the installer image must not contain whitespace")
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        let mut settings = state.settings.get_mut().await?;
        let extension = settings.find_mut_extension_settings::<ExtensionSettingsData>()?;
        extension.api_url = api_url;
        extension.installer_image = installer_image;
        extension.allow_clean_install = data.allow_clean_install;
        let response = AdminSettings::from(&*extension);
        settings.save().await?;

        activity_logger
            .log(
                "version-changer:settings.update",
                serde_json::json!({
                    "api_url": response.api_url,
                    "installer_image": response.installer_image,
                    "allow_clean_install": response.allow_clean_install,
                }),
            )
            .await;

        ApiResponse::new_serialized(Response { settings: response }).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .nest(
            "/settings",
            OpenApiRouter::new()
                .routes(routes!(get::route))
                .routes(routes!(put::route)),
        )
        .with_state(state.clone())
}
