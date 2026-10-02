import { z } from 'zod';
import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { type BuildSummary, buildSummarySchema } from '../../lib/schemas.ts';
import { serverVersionChangerBase } from '../paths.ts';

export default async (serverUuid: string, serverType: string, version: string): Promise<BuildSummary[]> => {
  const { data } = await axiosInstance.get(
    `${serverVersionChangerBase(serverUuid)}/types/${encodeURIComponent(serverType)}/versions/${encodeURIComponent(version)}/builds`,
  );
  return parseFromApi(z.array(buildSummarySchema), data.builds);
};
