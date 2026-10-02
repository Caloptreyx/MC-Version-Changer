import { z } from 'zod';
import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { type ServerType, serverTypeSchema } from '../../lib/schemas.ts';
import { serverVersionChangerBase } from '../paths.ts';

export default async (serverUuid: string): Promise<ServerType[]> => {
  const { data } = await axiosInstance.get(`${serverVersionChangerBase(serverUuid)}/types`);
  return parseFromApi(z.array(serverTypeSchema), data.types);
};
