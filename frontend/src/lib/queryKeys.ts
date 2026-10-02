const root = ['extensions', 'dev.caloptreyx.versionchanger'] as const;

export const versionChangerQueryKeys = {
  overview: (serverUuid: string) => [...root, 'server', serverUuid, 'overview'] as const,
  types: () => [...root, 'types'] as const,
  versions: (serverType: string) => [...root, 'types', serverType, 'versions'] as const,
  builds: (serverType: string, version: string) =>
    [...root, 'types', serverType, 'versions', version, 'builds'] as const,
  adminSettings: () => [...root, 'admin', 'settings'] as const,
};
