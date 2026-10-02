import { faMagnifyingGlass } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Group, SimpleGrid, Stack } from '@mantine/core';
import { useMemo, useState } from 'react';
import { useNavigate } from 'react-router';
import Badge from '@/elements/data-display/Badge.tsx';
import Card from '@/elements/data-display/Card.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import SegmentedControl from '@/elements/layout/SegmentedControl.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useServerStore } from '@/stores/server.ts';
import TypeIcon from '../../components/TypeIcon.tsx';
import type { Overview, ServerType } from '../../lib/schemas.ts';
import { matchesFilter, type TypeFilter, typeFilters, usesProjectVersions } from '../../lib/types.ts';
import { useExtTranslations } from '../../translations.ts';
import CurrentVersionCard from './CurrentVersionCard.tsx';
import type { InstallTarget } from './InstallVersionModal.tsx';

interface Props {
  overview: Overview;
  types: ServerType[];
  onInstall: (target: InstallTarget) => void;
}

function TypeCard({ type, current, onOpen }: { type: ServerType; current: boolean; onOpen: () => void }) {
  const { t: tExt } = useExtTranslations();
  const versions = usesProjectVersions(type) ? type.projectVersions : type.minecraftVersions;

  return (
    <Card hoverable onClick={onOpen} p='md' h='100%'>
      <Stack gap='sm' h='100%' justify='space-between'>
        <Group wrap='nowrap' align='flex-start' gap='md'>
          <TypeIcon url={type.icon} name={type.name} />
          <Stack gap={4} style={{ minWidth: 0, flex: 1 }}>
            <Group gap={6} wrap='wrap'>
              <Text fw={600} size='lg'>
                {type.name}
              </Text>
              {current && (
                <Badge color='green' variant='filled' size='xs'>
                  {tExt('pages.server.versionChanger.badges.current', {})}
                </Badge>
              )}
              {type.experimental && (
                <Badge color='yellow' variant='light' size='xs'>
                  {tExt('pages.server.versionChanger.badges.experimental', {})}
                </Badge>
              )}
              {type.deprecated && (
                <Badge color='red' variant='light' size='xs'>
                  {tExt('pages.server.versionChanger.badges.deprecated', {})}
                </Badge>
              )}
            </Group>
            <Text size='sm' c='dimmed' lineClamp={2}>
              {type.description}
            </Text>
          </Stack>
        </Group>
        <Group gap={6}>
          {type.categories.map((category) => (
            <Badge key={category} color='gray' variant='outline' size='sm'>
              {category}
            </Badge>
          ))}
          <Text size='xs' c='dimmed' ml='auto'>
            {tExt('pages.server.versionChanger.types.versions', { count: versions.toLocaleString() })} ·{' '}
            {tExt('pages.server.versionChanger.types.builds', { count: type.builds.toLocaleString() })}
          </Text>
        </Group>
      </Stack>
    </Card>
  );
}

export default function TypesView({ overview, types, onInstall }: Props) {
  const { t: tExt } = useExtTranslations();
  const navigate = useNavigate();
  const { server } = useServerStore();
  const [filter, setFilter] = useState<TypeFilter>('all');
  const [search, setSearch] = useState('');

  const visible = useMemo(() => {
    const query = search.trim().toLowerCase();
    return types.filter(
      (type) =>
        matchesFilter(type, filter) &&
        (!query || type.name.toLowerCase().includes(query) || type.id.toLowerCase().includes(query)),
    );
  }, [types, filter, search]);

  return (
    <Stack gap='md'>
      <CurrentVersionCard overview={overview} types={types} onInstall={onInstall} />

      <Group justify='space-between' wrap='wrap' gap='sm'>
        <SegmentedControl
          value={filter}
          onChange={(value) => setFilter(value as TypeFilter)}
          data={typeFilters.map((value) => ({
            value,
            label: tExt(`pages.server.versionChanger.types.filters.${value}`, {}),
          }))}
        />
        <TextInput
          placeholder={tExt('pages.server.versionChanger.types.searchPlaceholder', {})}
          leftSection={<FontAwesomeIcon icon={faMagnifyingGlass} />}
          value={search}
          onChange={(event) => setSearch(event.target.value)}
          w={{ base: '100%', sm: 260 }}
        />
      </Group>

      {visible.length === 0 ? (
        <EmptyState
          icon={faMagnifyingGlass}
          title={tExt('pages.server.versionChanger.types.empty.title', {})}
          description={tExt('pages.server.versionChanger.types.empty.description', {})}
        />
      ) : (
        <SimpleGrid cols={{ base: 1, sm: 2, xl: 3 }}>
          {visible.map((type) => (
            <TypeCard
              key={type.id}
              type={type}
              current={overview.current?.serverType === type.id}
              onOpen={() => navigate(`/server/${server.uuidShort}/version-changer/${type.id}`)}
            />
          ))}
        </SimpleGrid>
      )}
    </Stack>
  );
}
