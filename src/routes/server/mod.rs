use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod install;
mod types;

mod get {
    use crate::{detect::CurrentVersion, install::DockerImage};
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{server::GetServer, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        /// What the server runs now, `null` when neither its jar nor a marker identify it.
        #[schema(inline)]
        current: Option<CurrentVersion>,
        /// The jar the egg starts (`SERVER_JARFILE`).
        jar_file: String,
        allow_clean_install: bool,
        #[schema(inline)]
        docker_images: Vec<DockerImage>,
        current_image: String,
        /// Whether the user may switch the docker image while installing.
        can_change_image: bool,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
    ), params(
        (
            "server" = uuid::Uuid,
            description = "The server ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
    ) -> ApiResponseResult {
        permissions.has_server_permission("version-changer.read")?;

        let settings = crate::settings::load(&state).await?;
        let jar_file = crate::detect::jar_file(&state, &server).await;
        let current = crate::detect::detect(&state, &settings, &server, &jar_file).await;

        let image_listed = server.egg.docker_images.values().any(|image| *image == server.image);
        let can_change_image = permissions.has_server_permission("startup.docker-image").is_ok()
            && (image_listed
                || state
                    .settings
                    .get()
                    .await?
                    .server
                    .allow_overwriting_custom_docker_image);

        ApiResponse::new_serialized(Response {
            current,
            jar_file,
            allow_clean_install: settings.allow_clean_install,
            docker_images: crate::install::docker_images(server.egg.docker_images.iter()),
            current_image: server.image.to_string(),
            can_change_image,
        })
        .ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .nest("/types", types::router(state))
        .nest("/install", install::router(state))
        .with_state(state.clone())
}
