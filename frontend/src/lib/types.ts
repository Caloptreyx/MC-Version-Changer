import type { ServerType } from './schemas.ts';

export const typeFilters = ['all', 'servers', 'modded', 'proxies', 'other'] as const;
export type TypeFilter = (typeof typeFilters)[number];

export function matchesFilter(type: ServerType, filter: TypeFilter): boolean {
  const has = (category: string) => type.categories.includes(category);
  switch (filter) {
    case 'all':
      return true;
    case 'servers':
      return !has('proxy') && !has('limbo') && !has('modded');
    case 'modded':
      return has('modded');
    case 'proxies':
      return has('proxy');
    case 'other':
      return has('limbo');
  }
}

/** Types without Minecraft versions (proxies, some limbos) list project versions instead. */
export function usesProjectVersions(type: ServerType): boolean {
  return type.minecraftVersions === 0 && type.projectVersions > 0;
}

/** mcjars dates carry no zone; they are UTC. */
export function parseMcjarsDate(value: string | null): Date | null {
  if (!value) return null;
  const date = new Date(/[zZ]|[+-]\d\d:?\d\d$/.test(value) ? value : `${value}Z`);
  return Number.isNaN(date.getTime()) ? null : date;
}

/** Compares two release versions (`1.20.4`, `26.1`); null when either is not a plain release. */
export function compareReleases(a: string, b: string): number | null {
  const release = /^\d+(\.\d+)*$/;
  if (!release.test(a) || !release.test(b)) return null;

  const left = a.split('.').map(Number);
  const right = b.split('.').map(Number);
  for (let i = 0; i < Math.max(left.length, right.length); i++) {
    const diff = (left[i] ?? 0) - (right[i] ?? 0);
    if (diff !== 0) return diff;
  }
  return 0;
}
