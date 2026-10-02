import { faArrowUp, faCircleQuestion, faList } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Group, Stack } from '@mantine/core';
import { useNavigate } from 'react-router';
import Button from '@/elements/buttons/Button.tsx';
import { ServerCan } from '@/elements/Can.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Card from '@/elements/data-display/Card.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useServerStore } from '@/stores/server.ts';
import TypeIcon from '../../components/TypeIcon.tsx';
import type { Overview, ServerType } from '../../lib/schemas.ts';
import { parseMcjarsDate } from '../../lib/types.ts';
import { useExtTranslations } from '../../translations.ts';
import type { InstallTarget } from './InstallVersionModal.tsx';

interface Props {
  overview: Overview;
  types: ServerType[];
  onInstall: (target: InstallTarget) => void;
}

/** What the server runs now, with an update shortcut when a newer build of the same version exists. */
export default function CurrentVersionCard({ overview, types, onInstall }: Props) {
  const { t: tExt } = useExtTranslations();
  const navigate = useNavigate();
  const { server } = useServerStore();
  const { current } = overview;

  if (!current) {
    return (
      <Card p='md'>
        <Group wrap='nowrap' gap='md'>
          <TypeIcon url={null} name='?' />
          <Stack gap={2}>
            <Text fw={600}>{tExt('pages.server.versionChanger.current.unknown', {})}</Text>
            <Text size='sm' c='dimmed'>
              {tExt('pages.server.versionChanger.current.unknownDescription', { jar: overview.jarFile })}
            </Text>
          </Stack>
        </Group>
      </Card>
    );
  }

  const type = types.find((candidate) => candidate.id === current.serverType) ?? null;
  const name = type?.name ?? current.serverType;
  const installedAt = parseMcjarsDate(current.installedAt);

  return (
    <Card p='md'>
      <Group justify='space-between' wrap='wrap' gap='md'>
        <Group wrap='nowrap' gap='md' style={{ minWidth: 0 }}>
          <TypeIcon url={type?.icon ?? null} name={name} />
          <Stack gap={4} style={{ minWidth: 0 }}>
            <Text size='xs' c='dimmed' tt='uppercase' fw={600}>
              {tExt('pages.server.versionChanger.current.title', {})}
            </Text>
            <Group gap={8} wrap='wrap'>
              <Text fw={700} size='lg'>
                {current.version ? `${name} ${current.version}` : name}
              </Text>
              {current.buildName && (
                <Badge color='gray' variant='light'>
                  {tExt('pages.server.versionChanger.current.build', { build: current.buildName })}
                </Badge>
              )}
              <Tooltip
                label={tExt(`pages.server.versionChanger.current.source.${current.source}`, { jar: overview.jarFile })}
              >
                <Text c='dimmed' size='sm' component='span'>
                  <FontAwesomeIcon icon={faCircleQuestion} />
                </Text>
              </Tooltip>
            </Group>
            {installedAt && (
              <Text size='xs' c='dimmed' component='div'>
                {tExt('pages.server.versionChanger.current.installedAt', {})}{' '}
                <FormattedTimestamp timestamp={installedAt} />
              </Text>
            )}
            {current.update && (
              <Text size='sm' c='blue' component='div'>
                {tExt('pages.server.versionChanger.current.updateAvailable', { build: current.update.name })}
              </Text>
            )}
          </Stack>
        </Group>

        <Group gap='sm'>
          {type && current.version && current.update && (
            <ServerCan action='version-changer.install'>
              <Button
                leftSection={<FontAwesomeIcon icon={faArrowUp} />}
                onClick={() => onInstall({ type, version: current.version!, buildId: current.update!.id })}
              >
                {tExt('pages.server.versionChanger.current.update', {})}
              </Button>
            </ServerCan>
          )}
          {type && (
            <Button
              variant='default'
              leftSection={<FontAwesomeIcon icon={faList} />}
              onClick={() => navigate(`/server/${server.uuidShort}/version-changer/${type.id}`)}
            >
              {tExt('pages.server.versionChanger.current.browse', {})}
            </Button>
          )}
        </Group>
      </Group>
    </Card>
  );
}
