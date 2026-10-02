import { axiosInstance } from '@/api/axios.ts';
import { serializeForApi } from '@/lib/serialization/api-transform.ts';
import { type InstallVersion, installVersionSchema } from '../../lib/schemas.ts';
import { serverVersionChangerBase } from '../paths.ts';

export default async (serverUuid: string, data: InstallVersion): Promise<void> => {
  await axiosInstance.post(
    `${serverVersionChangerBase(serverUuid)}/install`,
    serializeForApi(installVersionSchema, data),
  );
};
