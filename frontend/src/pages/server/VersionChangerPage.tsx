import { Center, Stack } from '@mantine/core';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { Route, Routes } from 'react-router';
import { httpErrorToHuman } from '@/api/axios.ts';
import ServerContentContainer from '@/elements/containers/ServerContentContainer.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useServerStore } from '@/stores/server.ts';
import getOverview from '../../api/server/getOverview.ts';
import getTypes from '../../api/server/getTypes.ts';
import { versionChangerQueryKeys } from '../../lib/queryKeys.ts';
import { useExtTranslations } from '../../translations.ts';
import InstallVersionModal, { type InstallTarget } from './InstallVersionModal.tsx';
import TypesView from './TypesView.tsx';
import VersionsView from './VersionsView.tsx';

/** Server route `/version-changer/*`: the server type list and the versions of one type share the install modal. */
export default function VersionChangerPage() {
  const { t: tExt } = useExtTranslations();
  const { server } = useServerStore();
  const [installTarget, setInstallTarget] = useState<InstallTarget | null>(null);

  const overview = useQuery({
    queryKey: versionChangerQueryKeys.overview(server.uuid),
    queryFn: () => getOverview(server.uuid),
  });
  const types = useQuery({
    queryKey: versionChangerQueryKeys.types(),
    queryFn: () => getTypes(server.uuid),
    staleTime: 10 * 60 * 1000,
  });

  const error = overview.error ?? types.error;

  return (
    <ServerContentContainer title={tExt('pages.server.versionChanger.title', {})}>
      {error ? (
        <Alert color='red'>{httpErrorToHuman(error)}</Alert>
      ) : !overview.data || !types.data ? (
        <Center py='xl'>
          <Spinner />
        </Center>
      ) : (
        <Stack gap='md'>
          <InstallVersionModal
            target={installTarget}
            onClose={() => setInstallTarget(null)}
            overview={overview.data}
            types={types.data}
          />
          <Routes>
            <Route
              index
              element={<TypesView overview={overview.data} types={types.data} onInstall={setInstallTarget} />}
            />
            <Route
              path=':type'
              element={<VersionsView overview={overview.data} types={types.data} onInstall={setInstallTarget} />}
            />
          </Routes>
          <Text size='xs' c='dimmed' ta='center' component='div'>
            {tExt('pages.server.versionChanger.poweredBy', { url: 'https://mcjars.app' }).md()}
          </Text>
        </Stack>
      )}
    </ServerContentContainer>
  );
}
