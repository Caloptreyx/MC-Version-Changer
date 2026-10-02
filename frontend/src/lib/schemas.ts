import { z } from 'zod';

export const installModeSchema = z.enum(['replace', 'wipe']);
export type InstallMode = z.infer<typeof installModeSchema>;

export const serverTypeSchema = z.object({
  id: z.string(),
  name: z.string(),
  group: z.string(),
  icon: z.string(),
  color: z.string(),
  homepage: z.string(),
  description: z.string(),
  categories: z.array(z.string()),
  deprecated: z.boolean(),
  experimental: z.boolean(),
  builds: z.number(),
  minecraftVersions: z.number(),
  projectVersions: z.number(),
});

export type ServerType = z.infer<typeof serverTypeSchema>;

export const latestBuildSchema = z.object({
  id: z.number(),
  name: z.string(),
  experimental: z.boolean(),
});

export type LatestBuild = z.infer<typeof latestBuildSchema>;

export const versionSummarySchema = z.object({
  id: z.string(),
  snapshot: z.boolean(),
  supported: z.boolean(),
  java: z.number(),
  builds: z.number(),
  created: z.string().nullable(),
  latest: latestBuildSchema,
});

export type VersionSummary = z.infer<typeof versionSummarySchema>;

export const buildSummarySchema = z.object({
  id: z.number(),
  name: z.string(),
  buildNumber: z.number(),
  experimental: z.boolean(),
  minecraftVersion: z.string().nullable(),
  projectVersion: z.string().nullable(),
  created: z.string().nullable(),
  size: z.number().nullable(),
  archive: z.boolean(),
  installable: z.boolean(),
  changes: z.array(z.string()),
});

export type BuildSummary = z.infer<typeof buildSummarySchema>;

export const dockerImageSchema = z.object({
  name: z.string(),
  image: z.string(),
  javaVersion: z.number().nullable(),
});

export type DockerImage = z.infer<typeof dockerImageSchema>;

export const currentVersionSchema = z.object({
  serverType: z.string(),
  version: z.string().nullable(),
  buildId: z.number().nullable(),
  buildName: z.string().nullable(),
  source: z.enum(['jar', 'marker']),
  installedAt: z.string().nullable(),
  update: latestBuildSchema.nullable(),
});

export type CurrentVersion = z.infer<typeof currentVersionSchema>;

export const overviewSchema = z.object({
  current: currentVersionSchema.nullable(),
  jarFile: z.string(),
  allowCleanInstall: z.boolean(),
  dockerImages: z.array(dockerImageSchema),
  currentImage: z.string(),
  canChangeImage: z.boolean(),
});

export type Overview = z.infer<typeof overviewSchema>;

export const installVersionSchema = z.object({
  serverType: z.string(),
  version: z.string(),
  buildId: z.number(),
  mode: installModeSchema,
  acceptEula: z.boolean(),
  startOnCompletion: z.boolean(),
  dockerImage: z.string().optional(),
});

export type InstallVersion = z.infer<typeof installVersionSchema>;

export const adminSettingsSchema = z.object({
  apiUrl: z.string(),
  installerImage: z.string(),
  allowCleanInstall: z.boolean(),
});

export type AdminSettings = z.infer<typeof adminSettingsSchema>;

export const updateAdminSettingsSchema = z.object({
  apiUrl: z.string().min(1).max(255),
  installerImage: z.string().min(1).max(255),
  allowCleanInstall: z.boolean(),
});

export type UpdateAdminSettings = z.infer<typeof updateAdminSettingsSchema>;
