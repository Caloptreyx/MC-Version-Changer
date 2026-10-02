import { Center, Group, Stack } from '@mantine/core';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import { AdminCan } from '@/elements/Can.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import Switch from '@/elements/input/Switch.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import { useAdminCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import getSettings from '../../api/admin/getSettings.ts';
import updateSettings from '../../api/admin/updateSettings.ts';
import { versionChangerQueryKeys } from '../../lib/queryKeys.ts';
import type { AdminSettings } from '../../lib/schemas.ts';
import { useExtTranslations } from '../../translations.ts';

export default function AdminConfigurationPage() {
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();
  const queryClient = useQueryClient();
  const canManage = useAdminCan('version-changer.manage');

  const { data, isLoading, error } = useQuery({
    queryKey: versionChangerQueryKeys.adminSettings(),
    queryFn: getSettings,
  });

  const [settings, setSettings] = useState<AdminSettings | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (data) setSettings(data);
  }, [data]);

  if (isLoading) {
    return (
      <Center py='lg'>
        <Spinner />
      </Center>
    );
  }
  if (error || !settings) {
    return <Alert color='red'>{error ? httpErrorToHuman(error) : null}</Alert>;
  }

  const update = (patch: Partial<AdminSettings>) => setSettings({ ...settings, ...patch });

  const doSave = async () => {
    setSaving(true);
    try {
      const saved = await updateSettings(settings);
      setSettings(saved);
      queryClient.setQueryData(versionChangerQueryKeys.adminSettings(), saved);
      addToast(tExt('pages.admin.versionChanger.toast.saved', {}), 'success');
    } catch (err) {
      addToast(httpErrorToHuman(err), 'error');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Stack gap='md'>
      <TextInput
        withAsterisk
        label={tExt('pages.admin.versionChanger.apiUrl', {})}
        description={tExt('pages.admin.versionChanger.apiUrlDescription', {})}
        value={settings.apiUrl}
        disabled={!canManage}
        onChange={(event) => update({ apiUrl: event.target.value })}
      />
      <TextInput
        withAsterisk
        label={tExt('pages.admin.versionChanger.installerImage', {})}
        description={tExt('pages.admin.versionChanger.installerImageDescription', {})}
        value={settings.installerImage}
        disabled={!canManage}
        onChange={(event) => update({ installerImage: event.target.value })}
      />
      <Switch
        label={tExt('pages.admin.versionChanger.allowCleanInstall', {})}
        description={tExt('pages.admin.versionChanger.allowCleanInstallDescription', {})}
        checked={settings.allowCleanInstall}
        disabled={!canManage}
        onChange={(event) => update({ allowCleanInstall: event.currentTarget.checked })}
      />

      <Group justify='flex-end'>
        <AdminCan action='version-changer.manage' cantSave>
          <Button
            onClick={doSave}
            loading={saving}
            disabled={!settings.apiUrl.trim() || !settings.installerImage.trim()}
          >
            {tExt('pages.admin.versionChanger.button.save', {})}
          </Button>
        </AdminCan>
      </Group>
    </Stack>
  );
}
