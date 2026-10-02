import { faCodeBranch } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Extension, ExtensionContext } from 'shared';
import AdminConfigurationPage from './pages/admin/AdminConfigurationPage.tsx';
import VersionChangerPage from './pages/server/VersionChangerPage.tsx';
import { getExtTranslations } from './translations.ts';

class CaloptreyxVersionChangerExtension extends Extension {
  public cardConfigurationPage: React.FC | null = AdminConfigurationPage;
  public cardIcon: React.ReactNode = <FontAwesomeIcon icon={faCodeBranch} />;

  public initialize(ctx: ExtensionContext): void {
    ctx.extensionRegistry.enterRoutes((routes) =>
      routes.addServerRoute({
        name: () => getExtTranslations().t('pages.server.versionChanger.title', {}),
        icon: faCodeBranch,
        path: '/version-changer/*',
        element: VersionChangerPage,
        permission: 'version-changer.read',
      }),
    );

    ctx.extensionRegistry.enterPermissionIcons((icons) =>
      icons
        .addServerPermissionIcon('version-changer', <FontAwesomeIcon icon={faCodeBranch} />)
        .addAdminPermissionIcon('version-changer', <FontAwesomeIcon icon={faCodeBranch} />),
    );
  }
}

export default new CaloptreyxVersionChangerExtension();
