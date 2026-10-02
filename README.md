# MC Version Changer

A [Calagopus Panel](https://calagopus.com) extension that switches a Minecraft server to any server
type, version and build listed on [mcjars](https://mcjars.app) (Paper, Purpur, Folia, Fabric,
Quilt, Forge, NeoForge, Velocity, BungeeCord, Sponge, Mohist, Arclight, Leaf and more).

Package name: `dev.caloptreyx.versionchanger` · Requires panel `>=1.2.3`

## Features

- **Version Changer page** (`/server/<id>/version-changer`): a card grid of every server type
  mcjars offers, with filters (plugins & vanilla, modded, proxies, limbo) and search. Each type
  opens a grid of its versions with the Java version they need, their build count and latest
  build. Snapshots and pre-releases are hidden behind a toggle.
- **Install modal**: pick the build (newest stable build preselected), see its release date,
  download size and changelog, choose *Keep worlds & settings* or *Clean install*, switch the
  docker image, accept the EULA and optionally start the server when the install finishes.
  The modal warns when the server type changes, when the Minecraft version goes down, when the
  build is experimental and when the docker image's Java does not fit.
- **Java selection**: mcjars lists the Java release each version needs. The modal recommends
  the egg docker image that provides it and can switch to it (needs `startup.docker-image`).
- **Current version card**: shows what the server runs, identified by looking the server jar up on
  mcjars by SHA-256 (works for Forge/NeoForge too). When the jar is unknown, the
  `.mcvc-type.json` marker is used. A newer build of the same version is offered as a one-click
  update.

### Installation

Installs run through the panel's native Wings reinstall flow with a custom installation script, so
the server locks into the *installing* state, the console shows live progress and the stock
*Cancel* button works. The egg's own install script does not run.

- The backend resolves the build on mcjars itself (clients only send type, version and build id)
  and passes its installation steps to the script. Builds with unknown steps, non-HTTP(S) URLs or
  paths outside the server directory are refused.
- Every file is downloaded into a staging directory first; nothing on the server changes until all
  downloads succeeded.
- The mcjars steps then run in order (Forge/NeoForge builds replace `libraries/` and unpack a
  ready-made server). `server.jar` is renamed to the egg's `SERVER_JARFILE` when that differs.
- Forge/NeoForge eggs start with `@unix_args.txt` when it exists at the server root: the new
  build's copy is put there, and a stale one is removed when switching to any other type.
- **Keep worlds & settings** (default) leaves worlds, plugins, mods and configuration files alone.
  **Clean install** deletes every server file first; an administrator can disable it.
- `.mcvc-type.json` records the type, version and build. It uses the format of
  [MC Version Chooser](https://github.com/Regrave/mc-version-chooser), which the Modpack and Mod
  Installer extensions read to detect the loader.

## Permissions

| Permission | Allows |
| --- | --- |
| `version-changer.read` (server) | Viewing the detected version and browsing types, versions and builds |
| `version-changer.install` (server) | Installing a build (reinstalls the server) |
| `version-changer.read` (admin) | Viewing the extension settings |
| `version-changer.manage` (admin) | Changing the extension settings |

## Admin settings

- **mcjars URL**: defaults to `https://mcjars.app`; point it at a self-hosted mcjars instance if you
  run one.
- **Installer image**: the image the install script runs in. It needs `python3` 3.10 or newer;
  default `python:3.13-slim`.
- **Allow clean installs**.

## Notes

- Changing the version does not update egg variables such as `MINECRAFT_VERSION`. Reinstalling the
  server through the panel's own *Reinstall* button runs the egg script again and installs what
  those variables say.
- mcjars data is cached by the panel: the type list for an hour, version and build lists for five
  minutes, jar lookups for ten minutes.

## Development

The repository uses the root-crate layout (`Cargo.toml`, `src/`) next to `frontend/`. Every push
runs the shared Caloptreyx extension check (typecheck, Biome, frontend build and
`cargo test -p dev_caloptreyx_versionchanger` inside a panel checkout).

The install script also runs on its own: set `MVC_ROOT=/tmp/x`, `MVC_BUILD` (the JSON built by
`src/install/mod.rs`) and `MVC_JARFILE`, then run `python3 src/install/install.py`.

## License

MIT
