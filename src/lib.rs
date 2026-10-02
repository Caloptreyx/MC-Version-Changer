use indexmap::IndexMap;
use shared::{
    State,
    extensions::{
        Extension, ExtensionPermissionsBuilder, ExtensionRouteBuilder,
        settings::ExtensionSettingsDeserializer,
    },
    permissions::PermissionGroup,
};
use std::sync::Arc;

mod detect;
mod install;
mod mcjars;
mod routes;
mod settings;

#[derive(Default)]
pub struct ExtensionStruct;

#[async_trait::async_trait]
impl Extension for ExtensionStruct {
    async fn initialize_router(
        &mut self,
        state: State,
        builder: ExtensionRouteBuilder,
    ) -> ExtensionRouteBuilder {
        builder
            .add_admin_api_router(|router| {
                router.nest(
                    "/extensions/dev.caloptreyx.versionchanger",
                    routes::admin::router(&state),
                )
            })
            .add_client_server_api_router(|router| {
                router.nest("/version-changer", routes::server::router(&state))
            })
    }

    async fn initialize_permissions(
        &mut self,
        _state: State,
        mut builder: ExtensionPermissionsBuilder,
    ) -> ExtensionPermissionsBuilder {
        builder.server_permissions.insert(
            "version-changer",
            PermissionGroup {
                description: "Permissions that control the ability to change the Minecraft server type and version.",
                permissions: IndexMap::from([
                    (
                        "read",
                        "Allows viewing the detected server type and version and browsing the available versions.",
                    ),
                    (
                        "install",
                        "Allows changing the server type, version and build. This reinstalls the server and replaces its jar (and, optionally, all files).",
                    ),
                ]),
            },
        );

        builder.admin_permissions.insert(
            "version-changer",
            PermissionGroup {
                description: "Permissions that control the ability to configure the version changer extension.",
                permissions: IndexMap::from([
                    ("read", "Allows viewing the version changer settings."),
                    ("manage", "Allows changing the version changer settings."),
                ]),
            },
        );

        builder
    }

    async fn settings_deserializer(&self, _state: State) -> ExtensionSettingsDeserializer {
        Arc::new(settings::ExtensionSettingsDataDeserializer)
    }
}
