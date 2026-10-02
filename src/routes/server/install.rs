use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod post {
    use crate::{
        install::{InstallMode, ScriptOptions},
        mcjars::{self, user_error},
    };
    use axum::http::StatusCode;
    use garde::Validate;
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::{
            UpdatableModel,
            server::{GetServer, GetServerActivityLogger, ServerInstallOptions, UpdateServerOptions},
            user::GetPermissionManager,
        },
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Validate, Deserialize)]
    pub struct Payload {
        /// mcjars server type, e.g. `PAPER`.
        #[garde(length(min = 1, max = 32))]
        #[schema(min_length = 1, max_length = 32)]
        server_type: String,
        /// Minecraft version, or the project version for proxies and limbos.
        #[garde(length(min = 1, max = 64))]
        #[schema(min_length = 1, max_length = 64)]
        version: String,
        /// mcjars build id.
        #[garde(skip)]
        build_id: i32,
        #[garde(skip)]
        mode: InstallMode,
        /// Writes `eula=true` to eula.txt.
        #[garde(skip)]
        #[serde(default)]
        accept_eula: bool,
        /// Starts the server once the installation finished.
        #[garde(skip)]
        #[serde(default)]
        start_on_completion: bool,
        /// Switches the server to this docker image of its egg before installing.
        #[garde(length(chars, min = 2, max = 255))]
        #[schema(min_length = 2, max_length = 255)]
        docker_image: Option<String>,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {}

    #[utoipa::path(post, path = "/", responses(
        (status = ACCEPTED, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = FORBIDDEN, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
        (status = CONFLICT, body = ApiError),
        (status = EXPECTATION_FAILED, body = ApiError),
        (status = UNPROCESSABLE_ENTITY, body = ApiError),
        (status = BAD_GATEWAY, body = ApiError),
    ), params(
        (
            "server" = uuid::Uuid,
            description = "The server ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        mut server: GetServer,
        activity_logger: GetServerActivityLogger,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        if let Err(errors) = shared::utils::validate_data(&data) {
            return ApiResponse::new_serialized(ApiError::new_strings_value(errors))
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        permissions.has_server_permission("version-changer.install")?;

        if server.status.is_some() {
            return ApiResponse::error("the server is already installing or restoring a backup")
                .with_status(StatusCode::CONFLICT)
                .ok();
        }

        let settings = crate::settings::load(&state).await?;
        if data.mode == InstallMode::Wipe && !settings.allow_clean_install {
            return ApiResponse::error("clean installs have been disabled by an administrator")
                .with_status(StatusCode::FORBIDDEN)
                .ok();
        }

        // Resolve the build server-side: the client only names it.
        let build = mcjars::builds(&state, &settings, &data.server_type, &data.version)
            .await?
            .into_iter()
            .find(|build| build.id == data.build_id)
            .ok_or_else(|| user_error("build not found", StatusCode::NOT_FOUND))?;
        if !build.installable() {
            return ApiResponse::error("this build cannot be installed automatically")
                .with_status(StatusCode::UNPROCESSABLE_ENTITY)
                .ok();
        }

        let type_name = mcjars::types(&state, &settings)
            .await
            .ok()
            .and_then(|types| types.into_iter().find(|kind| kind.id == data.server_type))
            .map(|kind| kind.name)
            .unwrap_or_else(|| data.server_type.clone());

        let docker_image = data
            .docker_image
            .map(|image| image.trim().to_string())
            .filter(|image| *image != server.image);
        if let Some(image) = &docker_image {
            permissions.has_server_permission("startup.docker-image")?;

            if !server.egg.docker_images.values().any(|egg_image| egg_image == image) {
                return ApiResponse::error("the specified docker image is not available")
                    .with_status(StatusCode::EXPECTATION_FAILED)
                    .ok();
            }
            if !state.settings.get().await?.server.allow_overwriting_custom_docker_image
                && !server.egg.docker_images.values().any(|egg_image| *egg_image == server.image)
            {
                return ApiResponse::error("overwriting custom docker images is not allowed")
                    .with_status(StatusCode::EXPECTATION_FAILED)
                    .ok();
            }
        }

        let jar_file = crate::detect::jar_file(&state, &server).await;
        let script = crate::install::script(ScriptOptions {
            build: &build,
            type_name: &type_name,
            jar_file: &jar_file,
            mode: data.mode,
            accept_eula: data.accept_eula,
            image: &settings.installer_image,
        });

        // Run to completion even if the client disconnects: the install
        // commits the server status and starts the Wings reinstall together.
        tokio::spawn(async move {
            if let Some(image) = &docker_image {
                server
                    .update(
                        &state,
                        UpdateServerOptions {
                            image: Some(image.as_str().into()),
                            ..Default::default()
                        },
                    )
                    .await?;
            }

            server
                .install_with_options(
                    &state,
                    ServerInstallOptions {
                        truncate_directory: data.mode == InstallMode::Wipe,
                        installation_script: Some(script),
                        start_on_completion: data.start_on_completion,
                    },
                )
                .await?;

            activity_logger
                .log(
                    "server:version-changer.install",
                    serde_json::json!({
                        "type": build.server_type,
                        "type_name": type_name,
                        "version": build.version(),
                        "build_id": build.id,
                        "build": build.name,
                        "mode": data.mode,
                        "accept_eula": data.accept_eula,
                        "start_on_completion": data.start_on_completion,
                        "docker_image": docker_image,
                    }),
                )
                .await;

            if docker_image.is_some() {
                server.0.batch_sync(&state.database).await;
            }

            ApiResponse::new_serialized(Response {})
                .with_status(StatusCode::ACCEPTED)
                .ok()
        })
        .await?
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(post::route))
        .with_state(state.clone())
}
