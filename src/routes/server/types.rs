use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod list {
    use crate::mcjars::ServerType;
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
        types: Vec<ServerType>,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
        (status = BAD_GATEWAY, body = ApiError),
    ), params(
        (
            "server" = uuid::Uuid,
            description = "The server ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
    ))]
    pub async fn route(state: GetState, permissions: GetPermissionManager) -> ApiResponseResult {
        permissions.has_server_permission("version-changer.read")?;

        let settings = crate::settings::load(&state).await?;
        let types = crate::mcjars::types(&state, &settings).await?;

        ApiResponse::new_serialized(Response { types }).ok()
    }
}

mod versions {
    use crate::mcjars::VersionSummary;
    use axum::extract::Path;
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
        versions: Vec<VersionSummary>,
    }

    #[utoipa::path(get, path = "/{type}/versions", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
        (status = BAD_GATEWAY, body = ApiError),
    ), params(
        (
            "server" = uuid::Uuid,
            description = "The server ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
        ("type" = String, description = "The mcjars server type", example = "PAPER"),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        Path((_server, server_type)): Path<(String, String)>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("version-changer.read")?;

        let settings = crate::settings::load(&state).await?;
        let versions = crate::mcjars::versions(&state, &settings, &server_type).await?;

        ApiResponse::new_serialized(Response { versions }).ok()
    }
}

mod builds {
    use crate::mcjars::BuildSummary;
    use axum::extract::Path;
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
        builds: Vec<BuildSummary>,
    }

    #[utoipa::path(get, path = "/{type}/versions/{version}/builds", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
        (status = BAD_GATEWAY, body = ApiError),
    ), params(
        (
            "server" = uuid::Uuid,
            description = "The server ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
        ("type" = String, description = "The mcjars server type", example = "PAPER"),
        (
            "version" = String,
            description = "The Minecraft version, or the project version for proxies",
            example = "1.21.1",
        ),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        Path((_server, server_type, version)): Path<(String, String, String)>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("version-changer.read")?;

        let settings = crate::settings::load(&state).await?;
        let builds = crate::mcjars::builds(&state, &settings, &server_type, &version)
            .await?
            .iter()
            .map(|build| build.summary())
            .collect();

        ApiResponse::new_serialized(Response { builds }).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(list::route))
        .routes(routes!(versions::route))
        .routes(routes!(builds::route))
        .with_state(state.clone())
}
