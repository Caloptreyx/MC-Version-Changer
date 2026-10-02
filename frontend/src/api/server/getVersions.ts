import { z } from 'zod';
import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { type VersionSummary, versionSummarySchema } from '../../lib/schemas.ts';
import { serverVersionChangerBase } from '../paths.ts';

export default async (serverUuid: string, serverType: string): Promise<VersionSummary[]> => {
  const { data } = await axiosInstance.get(
    `${serverVersionChangerBase(serverUuid)}/types/${encodeURIComponent(serverType)}/versions`,
  );
  return parseFromApi(z.array(versionSummarySchema), data.versions);
};
