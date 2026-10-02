import { faDownload, faTriangleExclamation } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Group, List, ScrollArea, Stack } from '@mantine/core';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Checkbox from '@/elements/input/Checkbox.tsx';
import Select from '@/elements/input/Select.tsx';
import SegmentedControl from '@/elements/layout/SegmentedControl.tsx';
import { Modal, ModalFooter } from '@/elements/modals/Modal.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Text from '@/elements/typography/Text.tsx';
import { bytesToString } from '@/lib/format/size.ts';
import { useServerCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import { useServerStore } from '@/stores/server.ts';
import getBuilds from '../../api/server/getBuilds.ts';
import getVersions from '../../api/server/getVersions.ts';
import installVersion from '../../api/server/installVersion.ts';
import TypeIcon from '../../components/TypeIcon.tsx';
import { javaMismatch, recommendedImage } from '../../lib/java.ts';
import { versionChangerQueryKeys } from '../../lib/queryKeys.ts';
import type { BuildSummary, InstallMode, Overview, ServerType } from '../../lib/schemas.ts';
import { compareReleases, parseMcjarsDate } from '../../lib/types.ts';
import { useExtTranslations } from '../../translations.ts';

export interface InstallTarget {
  type: ServerType;
  version: string;
  /** Preselected build; the newest stable build otherwise. */
  buildId: number | null;
}

interface Props {
  target: InstallTarget | null;
  onClose: () => void;
  overview: Overview;
  types: ServerType[];
}

function defaultBuild(builds: BuildSummary[], preset: number | null): BuildSummary | null {
  const installable = builds.filter((build) => build.installable);
  return (
    installable.find((build) => build.id === preset) ??
    installable.find((build) => !build.experimental) ??
    installable[0] ??
    null
  );
}

export default function InstallVersionModal({ target, onClose, overview, types }: Props) {
  const { t } = useTranslations();
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { server, updateServer } = useServerStore();
  const canChangeImagePermission = useServerCan('startup.docker-image');

  const [buildId, setBuildId] = useState<number | null>(null);
  const [mode, setMode] = useState<InstallMode>('replace');
  const [acceptEula, setAcceptEula] = useState(false);
  const [startOnCompletion, setStartOnCompletion] = useState(false);
  const [dockerImage, setDockerImage] = useState<string | null>(null);
  const [imageTouched, setImageTouched] = useState(false);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!target) return;

    setBuildId(null);
    setMode('replace');
    setAcceptEula(false);
    setStartOnCompletion(false);
    setDockerImage(null);
    setImageTouched(false);
  }, [target]);

  const versions = useQuery({
    queryKey: versionChangerQueryKeys.versions(target?.type.id ?? ''),
    queryFn: () => getVersions(server.uuid, target!.type.id),
    enabled: !!target,
  });
  const builds = useQuery({
    queryKey: versionChangerQueryKeys.builds(target?.type.id ?? '', target?.version ?? ''),
    queryFn: () => getBuilds(server.uuid, target!.type.id, target!.version),
    enabled: !!target,
  });

  useEffect(() => {
    if (!builds.data || !target) return;
    if (buildId === null || !builds.data.some((build) => build.id === buildId)) {
      setBuildId(defaultBuild(builds.data, target.buildId)?.id ?? null);
    }
  }, [builds.data, target, buildId]);

  const build = builds.data?.find((candidate) => candidate.id === buildId) ?? null;
  const java = versions.data?.find((version) => version.id === target?.version)?.java || null;
  const current = overview.current;

  const canChangeImage = overview.canChangeImage && canChangeImagePermission && overview.dockerImages.length > 1;
  const recommended = java ? recommendedImage(overview.dockerImages, overview.currentImage, java) : null;
  const chosenImage = overview.dockerImages.find(
    (image) => image.image === (canChangeImage ? (dockerImage ?? overview.currentImage) : overview.currentImage),
  );

  useEffect(() => {
    if (!imageTouched) setDockerImage(recommended?.image ?? overview.currentImage);
  }, [recommended?.image, imageTouched, overview.currentImage]);

  const imageMismatch =
    java !== null && chosenImage?.javaVersion != null && javaMismatch(java, chosenImage.javaVersion);
  const isProxy = !!target && target.type.categories.includes('proxy');
  const currentTypeName = current
    ? (types.find((type) => type.id === current.serverType)?.name ?? current.serverType)
    : null;
  const typeChanged = !!target && !!current && current.serverType !== target.type.id;
  const currentIsProxy =
    !!current && !!types.find((type) => type.id === current.serverType)?.categories.includes('proxy');
  const downgrade =
    !!target &&
    !!current?.version &&
    !isProxy &&
    !currentIsProxy &&
    (compareReleases(target.version, current.version) ?? 0) < 0;

  const buildOptions = useMemo(
    () =>
      (builds.data ?? []).map((candidate, index) => {
        const key =
          current?.buildId === candidate.id
            ? 'buildOptionInstalled'
            : index === 0
              ? 'buildOptionLatest'
              : candidate.experimental
                ? 'buildOptionExperimental'
                : 'buildOption';
        return {
          value: String(candidate.id),
          label: tExt(`pages.server.versionChanger.install.${key}`, { build: candidate.name }),
          disabled: !candidate.installable,
        };
      }),
    [builds.data, current?.buildId],
  );

  const imageOptions = overview.dockerImages.map((image) => ({
    value: image.image,
    label:
      image.image === overview.currentImage
        ? tExt('pages.server.versionChanger.install.dockerImageCurrent', { name: image.name })
        : image.image === recommended?.image
          ? tExt('pages.server.versionChanger.install.dockerImageRecommended', { name: image.name })
          : image.name,
  }));

  const released = parseMcjarsDate(build?.created ?? null);

  const doInstall = async () => {
    if (!target || !build) return;

    setLoading(true);
    try {
      await installVersion(server.uuid, {
        serverType: target.type.id,
        version: target.version,
        buildId: build.id,
        mode,
        acceptEula: acceptEula && !isProxy,
        startOnCompletion,
        dockerImage: canChangeImage && dockerImage && dockerImage !== overview.currentImage ? dockerImage : undefined,
      });

      addToast(
        tExt('pages.server.versionChanger.install.toast', { type: target.type.name, version: target.version }),
        'success',
      );
      queryClient.invalidateQueries({ queryKey: versionChangerQueryKeys.overview(server.uuid) });
      onClose();
      navigate(`/server/${server.uuidShort}`);
      updateServer({ status: 'installing' });
    } catch (error) {
      addToast(httpErrorToHuman(error), 'error');
    } finally {
      setLoading(false);
    }
  };

  const error = versions.error ?? builds.error;

  return (
    <Modal
      opened={!!target}
      onClose={onClose}
      size='lg'
      title={
        target ? (
          <Group gap='sm' wrap='nowrap'>
            <TypeIcon url={target.type.icon} name={target.type.name} size={32} />
            <Text fw={600}>
              {tExt('pages.server.versionChanger.install.title', {
                type: target.type.name,
                version: target.version,
              })}
            </Text>
          </Group>
        ) : null
      }
    >
      <Stack gap='md'>
        {error && <Alert color='red'>{httpErrorToHuman(error)}</Alert>}

        <Select
          withAsterisk
          searchable
          label={tExt('pages.server.versionChanger.install.build', {})}
          data={buildOptions}
          value={buildId === null ? null : String(buildId)}
          onChange={(value) => setBuildId(value === null ? null : Number(value))}
          disabled={!builds.data || buildOptions.length === 0}
          error={
            builds.data && buildOptions.length === 0
              ? tExt('pages.server.versionChanger.install.noBuilds', {})
              : undefined
          }
        />

        {build && (
          <Stack gap={6}>
            <Group gap='md'>
              {released && (
                <Text size='xs' c='dimmed' component='div'>
                  {tExt('pages.server.versionChanger.install.released', {})} <FormattedTimestamp timestamp={released} />
                </Text>
              )}
              {build.size !== null && (
                <Text size='xs' c='dimmed'>
                  {tExt('pages.server.versionChanger.install.size', { size: bytesToString(build.size) })}
                </Text>
              )}
            </Group>
            {build.archive && (
              <Text size='xs' c='dimmed'>
                {tExt('pages.server.versionChanger.install.archive', {})}
              </Text>
            )}
            <Text size='sm' fw={500}>
              {tExt('pages.server.versionChanger.install.changes', {})}
            </Text>
            {build.changes.length === 0 ? (
              <Text size='xs' c='dimmed'>
                {tExt('pages.server.versionChanger.install.noChanges', {})}
              </Text>
            ) : (
              <ScrollArea.Autosize mah={140}>
                <List size='xs' spacing={2}>
                  {build.changes.map((change, index) => (
                    <List.Item key={index}>{change}</List.Item>
                  ))}
                </List>
              </ScrollArea.Autosize>
            )}
          </Stack>
        )}

        <Stack gap={6}>
          <Text size='sm' fw={500}>
            {tExt('pages.server.versionChanger.install.mode', {})}
          </Text>
          <SegmentedControl
            fullWidth
            value={mode}
            onChange={(value) => setMode(value as InstallMode)}
            data={[
              { value: 'replace', label: tExt('pages.server.versionChanger.install.modes.replace', {}) },
              {
                value: 'wipe',
                label: tExt('pages.server.versionChanger.install.modes.wipe', {}),
                disabled: !overview.allowCleanInstall,
              },
            ]}
          />
          <Text size='xs' c={mode === 'wipe' ? 'red' : 'dimmed'} component='div'>
            {mode === 'wipe'
              ? tExt('pages.server.versionChanger.install.modes.wipeDescription', {}).md()
              : overview.allowCleanInstall
                ? tExt('pages.server.versionChanger.install.modes.replaceDescription', { jar: overview.jarFile })
                : `${tExt('pages.server.versionChanger.install.modes.replaceDescription', { jar: overview.jarFile })} ${tExt('pages.server.versionChanger.install.modes.wipeDisabled', {})}`}
          </Text>
        </Stack>

        {canChangeImage && (
          <Select
            label={tExt('pages.server.versionChanger.install.dockerImage', {})}
            description={
              java && target
                ? tExt('pages.server.versionChanger.install.dockerImageDescription', {
                    type: target.type.name,
                    version: target.version,
                    java,
                  })
                : undefined
            }
            data={imageOptions}
            value={dockerImage ?? overview.currentImage}
            onChange={(value) => {
              setImageTouched(true);
              setDockerImage(value);
            }}
          />
        )}

        <Stack gap='xs'>
          {!isProxy && (
            <Checkbox
              checked={acceptEula}
              onChange={(event) => setAcceptEula(event.currentTarget.checked)}
              label={tExt('pages.server.versionChanger.install.acceptEula', {}).md()}
            />
          )}
          <Checkbox
            checked={startOnCompletion}
            onChange={(event) => setStartOnCompletion(event.currentTarget.checked)}
            label={tExt('pages.server.versionChanger.install.startOnCompletion', {})}
          />
        </Stack>

        {(typeChanged || downgrade || build?.experimental || imageMismatch || build?.installable === false) && (
          <Alert color='yellow' icon={<FontAwesomeIcon icon={faTriangleExclamation} />}>
            <Stack gap={4}>
              {build?.installable === false && (
                <Text size='sm'>{tExt('pages.server.versionChanger.install.warnings.notInstallable', {})}</Text>
              )}
              {typeChanged && target && currentTypeName && (
                <Text size='sm' component='div'>
                  {tExt('pages.server.versionChanger.install.warnings.typeChange', {
                    current: currentTypeName,
                    next: target.type.name,
                  }).md()}
                </Text>
              )}
              {downgrade && target && current?.version && (
                <Text size='sm' component='div'>
                  {tExt('pages.server.versionChanger.install.warnings.downgrade', {
                    current: current.version,
                    next: target.version,
                  }).md()}
                </Text>
              )}
              {build?.experimental && (
                <Text size='sm'>{tExt('pages.server.versionChanger.install.warnings.experimental', {})}</Text>
              )}
              {imageMismatch && java && target && chosenImage?.javaVersion != null && (
                <Text size='sm'>
                  {tExt('pages.server.versionChanger.install.warnings.javaMismatch', {
                    current: chosenImage.javaVersion,
                    version: `${target.type.name} ${target.version}`,
                    java,
                  })}
                </Text>
              )}
            </Stack>
          </Alert>
        )}

        <Text size='xs' c='dimmed'>
          {tExt('pages.server.versionChanger.install.notice', {})}
        </Text>

        <ModalFooter>
          <Button
            color={mode === 'wipe' ? 'red' : 'blue'}
            leftSection={<FontAwesomeIcon icon={faDownload} />}
            loading={loading || builds.isLoading}
            disabled={!build || !build.installable}
            onClick={doInstall}
          >
            {tExt('pages.server.versionChanger.install.submit', {})}
          </Button>
          <Button variant='default' onClick={onClose}>
            {t('common.button.cancel', {})}
          </Button>
        </ModalFooter>
      </Stack>
    </Modal>
  );
}
