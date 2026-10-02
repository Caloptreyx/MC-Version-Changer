import { faArrowLeft, faArrowUpRightFromSquare, faMagnifyingGlass } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Center, Group, SimpleGrid, Stack } from '@mantine/core';
import { useQuery } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Card from '@/elements/data-display/Card.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import Switch from '@/elements/input/Switch.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Anchor from '@/elements/typography/Anchor.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useServerCan } from '@/plugins/usePermissions.ts';
import { useServerStore } from '@/stores/server.ts';
import getVersions from '../../api/server/getVersions.ts';
import TypeIcon from '../../components/TypeIcon.tsx';
import { versionChangerQueryKeys } from '../../lib/queryKeys.ts';
import type { Overview, ServerType, VersionSummary } from '../../lib/schemas.ts';
import { useExtTranslations } from '../../translations.ts';
import type { InstallTarget } from './InstallVersionModal.tsx';

interface Props {
  overview: Overview;
  types: ServerType[];
  onInstall: (target: InstallTarget) => void;
}

function VersionTile({
  version,
  current,
  onSelect,
}: {
  version: VersionSummary;
  current: boolean;
  onSelect: (() => void) | null;
}) {
  const { t: tExt } = useExtTranslations();

  return (
    <Card hoverable={!!onSelect} onClick={onSelect ?? undefined} p='sm'>
      <Stack gap={4}>
        <Group gap={6} wrap='nowrap' justify='space-between'>
          <Text fw={600} truncate>
            {version.id}
          </Text>
          {current && (
            <Badge color='green' variant='filled' size='xs'>
              {tExt('pages.server.versionChanger.badges.current', {})}
            </Badge>
          )}
        </Group>
        <Group gap={4}>
          {version.java > 0 && (
            <Badge color='blue' variant='light' size='xs'>
              {tExt('pages.server.versionChanger.badges.java', { java: version.java })}
            </Badge>
          )}
          {version.snapshot && (
            <Badge color='yellow' variant='light' size='xs'>
              {tExt('pages.server.versionChanger.badges.snapshot', {})}
            </Badge>
          )}
          {!version.supported && (
            <Badge color='gray' variant='light' size='xs'>
              {tExt('pages.server.versionChanger.badges.unsupported', {})}
            </Badge>
          )}
        </Group>
        <Text size='xs' c='dimmed' truncate>
          {tExt('pages.server.versionChanger.versions.builds', { count: version.builds.toLocaleString() })} ·{' '}
          {tExt('pages.server.versionChanger.versions.latestBuild', { build: version.latest.name })}
        </Text>
      </Stack>
    </Card>
  );
}

export default function VersionsView({ overview, types, onInstall }: Props) {
  const { t: tExt } = useExtTranslations();
  const navigate = useNavigate();
  const params = useParams();
  const { server } = useServerStore();
  const canInstall = useServerCan('version-changer.install');
  const [search, setSearch] = useState('');
  const [showSnapshots, setShowSnapshots] = useState(false);

  const type = types.find((candidate) => candidate.id === params.type) ?? null;

  const versions = useQuery({
    queryKey: versionChangerQueryKeys.versions(type?.id ?? ''),
    queryFn: () => getVersions(server.uuid, type!.id),
    enabled: !!type,
  });

  const list = versions.data ?? [];
  const hasReleases = list.some((version) => !version.snapshot);
  const hasSnapshots = list.some((version) => version.snapshot);

  const visible = useMemo(() => {
    const query = search.trim().toLowerCase();
    return list.filter(
      (version) =>
        (showSnapshots || !hasReleases || !version.snapshot) && (!query || version.id.toLowerCase().includes(query)),
    );
  }, [list, search, showSnapshots, hasReleases]);

  const back = (
    <Button
      variant='subtle'
      leftSection={<FontAwesomeIcon icon={faArrowLeft} />}
      onClick={() => navigate(`/server/${server.uuidShort}/version-changer`)}
      w='fit-content'
    >
      {tExt('pages.server.versionChanger.versions.back', {})}
    </Button>
  );

  if (!type) {
    return (
      <Stack gap='md'>
        {back}
        <EmptyState
          icon={faMagnifyingGlass}
          title={tExt('pages.server.versionChanger.types.empty.title', {})}
          description={tExt('pages.server.versionChanger.types.empty.description', {})}
        />
      </Stack>
    );
  }

  const currentVersion = overview.current?.serverType === type.id ? overview.current.version : null;

  return (
    <Stack gap='md'>
      {back}

      <Group wrap='nowrap' align='flex-start' gap='md'>
        <TypeIcon url={type.icon} name={type.name} size={56} />
        <Stack gap={4} style={{ minWidth: 0 }}>
          <Group gap={8}>
            <Text fw={700} size='xl'>
              {type.name}
            </Text>
            {type.experimental && (
              <Badge color='yellow' variant='light' size='sm'>
                {tExt('pages.server.versionChanger.badges.experimental', {})}
              </Badge>
            )}
            {type.deprecated && (
              <Badge color='red' variant='light' size='sm'>
                {tExt('pages.server.versionChanger.badges.deprecated', {})}
              </Badge>
            )}
          </Group>
          <Text size='sm' c='dimmed'>
            {type.description}
          </Text>
          {type.homepage && (
            <Anchor href={type.homepage} target='_blank' rel='noreferrer' size='sm'>
              {tExt('pages.server.versionChanger.versions.homepage', {})}{' '}
              <FontAwesomeIcon icon={faArrowUpRightFromSquare} size='xs' />
            </Anchor>
          )}
        </Stack>
      </Group>

      <Group justify='space-between' wrap='wrap' gap='sm'>
        <TextInput
          placeholder={tExt('pages.server.versionChanger.versions.searchPlaceholder', {})}
          leftSection={<FontAwesomeIcon icon={faMagnifyingGlass} />}
          value={search}
          onChange={(event) => setSearch(event.target.value)}
          w={{ base: '100%', sm: 260 }}
        />
        {hasReleases && hasSnapshots && (
          <Switch
            label={tExt('pages.server.versionChanger.versions.showSnapshots', {})}
            checked={showSnapshots}
            onChange={(event) => setShowSnapshots(event.currentTarget.checked)}
          />
        )}
      </Group>

      {versions.error ? (
        <Alert color='red'>{httpErrorToHuman(versions.error)}</Alert>
      ) : versions.isLoading ? (
        <Center py='xl'>
          <Spinner />
        </Center>
      ) : visible.length === 0 ? (
        <EmptyState
          icon={faMagnifyingGlass}
          title={tExt('pages.server.versionChanger.versions.empty.title', {})}
          description={tExt('pages.server.versionChanger.versions.empty.description', {})}
        />
      ) : (
        <SimpleGrid cols={{ base: 2, sm: 3, md: 4, xl: 6 }}>
          {visible.map((version) => (
            <VersionTile
              key={version.id}
              version={version}
              current={version.id === currentVersion}
              onSelect={canInstall ? () => onInstall({ type, version: version.id, buildId: null }) : null}
            />
          ))}
        </SimpleGrid>
      )}
    </Stack>
  );
}
