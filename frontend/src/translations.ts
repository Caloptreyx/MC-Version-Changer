import { defineEnglishItem, defineTranslations } from 'shared';

const translations = defineTranslations({
  items: {
    version: defineEnglishItem('Version', 'Versions'),
    build: defineEnglishItem('Build', 'Builds'),
  },
  translations: {
    pages: {
      server: {
        versionChanger: {
          title: 'Version Changer',
          poweredBy: 'Versions from [mcjars]({url})',
          current: {
            title: 'Currently installed',
            unknown: 'Unknown server version',
            unknownDescription:
              'The server jar ({jar}) does not match any build on mcjars. Pick a server type below to install one.',
            build: 'Build {build}',
            installedAt: 'Installed',
            source: {
              jar: 'Identified from {jar}',
              marker: 'Read from .mcvc-type.json',
            },
            updateAvailable: 'Build {build} is available',
            update: 'Update',
            browse: 'Other versions',
          },
          types: {
            searchPlaceholder: 'Search server types…',
            filters: {
              all: 'All',
              servers: 'Plugins & vanilla',
              modded: 'Modded',
              proxies: 'Proxies',
              other: 'Limbo',
            },
            versions: '{count} versions',
            builds: '{count} builds',
            empty: {
              title: 'No server types found',
              description: 'Try a different search term or filter.',
            },
          },
          badges: {
            current: 'Current',
            experimental: 'Experimental',
            deprecated: 'Deprecated',
            snapshot: 'Snapshot',
            unsupported: 'Unsupported',
            latest: 'Latest',
            java: 'Java {java}',
          },
          versions: {
            back: 'All server types',
            homepage: 'Homepage',
            searchPlaceholder: 'Search versions…',
            showSnapshots: 'Show snapshots and pre-releases',
            builds: '{count} builds',
            latestBuild: 'Latest: {build}',
            empty: {
              title: 'No versions found',
              description: 'Try a different search term or show snapshots.',
            },
          },
          install: {
            title: 'Install {type} {version}',
            build: 'Build',
            buildOption: '{build}',
            buildOptionLatest: '{build} (latest)',
            buildOptionExperimental: '{build} (experimental)',
            buildOptionInstalled: '{build} (installed)',
            noBuilds: 'This version has no builds.',
            released: 'Released',
            size: 'Download size {size}',
            archive: 'Installed from an archive that replaces the libraries folder.',
            changes: 'Changes',
            noChanges: 'No changelog for this build.',
            mode: 'Installation',
            modes: {
              replace: 'Keep worlds & settings',
              wipe: 'Clean install',
              replaceDescription:
                'Replaces the server jar ({jar}). Worlds, plugins, mods and configuration files stay.',
              wipeDescription: '**Deletes every file on the server**, including worlds, before installing.',
              wipeDisabled: 'Clean installs have been disabled by an administrator.',
            },
            dockerImage: 'Docker image',
            dockerImageDescription: '{type} {version} needs Java {java}.',
            dockerImageCurrent: '{name} (current)',
            dockerImageRecommended: '{name} (recommended)',
            acceptEula: 'I accept the [Minecraft EULA](https://aka.ms/MinecraftEULA)',
            startOnCompletion: 'Start the server when the installation finishes',
            warnings: {
              typeChange:
                'This switches the server from **{current}** to **{next}**. Plugins and mods are not converted and may stop working.',
              downgrade:
                'Minecraft cannot load worlds saved by a newer version (**{current}** to **{next}**). Back up the server first.',
              experimental: 'This build is marked experimental by mcjars.',
              javaMismatch:
                'The selected docker image provides Java {current}, but {version} needs Java {java}. The server may fail to start.',
              notInstallable: 'This build cannot be installed automatically.',
            },
            notice:
              'The server stops and reinstalls; the console shows the progress. Your egg install script does not run.',
            submit: 'Install',
            toast: 'Installing {type} {version}…',
          },
        },
      },
      admin: {
        versionChanger: {
          apiUrl: 'mcjars URL',
          apiUrlDescription:
            'Base URL of the mcjars instance the versions come from, e.g. https://mcjars.app. Change it only for a self-hosted mirror.',
          installerImage: 'Installer image',
          installerImageDescription:
            'Docker image the installation runs in. It needs python3 (3.10 or newer); python:3.13-slim is the default.',
          allowCleanInstall: 'Allow clean installs',
          allowCleanInstallDescription: 'Lets users delete every server file while changing the version.',
          button: {
            save: 'Save',
          },
          toast: {
            saved: 'Settings saved.',
          },
        },
      },
    },
  },
});

export const useExtTranslations = translations.useTranslations.bind(translations);
export const getExtTranslations = translations.getTranslations.bind(translations);

export default translations;
