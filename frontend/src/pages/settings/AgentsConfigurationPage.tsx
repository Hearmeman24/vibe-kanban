import { useState, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Loader2, Upload, CheckCircle2 } from 'lucide-react';
import { useProfiles } from '@/hooks/useProfiles';
import { UserAvatar } from '@/components/tasks/UserAvatar';
import { AvatarUploadDialog } from '@/components/dialogs/settings/AvatarUploadDialog';
import { BaseCodingAgent } from 'shared/types';

interface AgentAvatarState {
  url: string | null;
  uploading: boolean;
  success: boolean;
  error: string | null;
}

type AgentAvatars = Record<string, AgentAvatarState>;

export function AgentsConfigurationPage() {
  const { t } = useTranslation(['settings', 'common']);
  const { parsedProfiles, isLoading, isError, error } = useProfiles();

  // Local state for avatar URLs (can be extended to localStorage later)
  const [avatars, setAvatars] = useState<AgentAvatars>({});
  const [uploadSuccess, setUploadSuccess] = useState<string | null>(null);

  // Extract agent list from profiles
  const agents = useMemo(() => {
    if (!parsedProfiles || typeof parsedProfiles !== 'object') {
      return [];
    }

    const profiles = parsedProfiles as { executors?: Record<string, unknown> };
    if (!profiles.executors) {
      return [];
    }

    return Object.keys(profiles.executors).map((agentKey) => ({
      id: agentKey,
      name: agentKey,
      role: agentKey as BaseCodingAgent,
    }));
  }, [parsedProfiles]);

  // Get display name for agent role
  const getAgentRoleDisplayName = useCallback(
    (role: BaseCodingAgent): string => {
      const roleKey = `settings.agentsConfiguration.roles.${role}`;
      const translated = t(roleKey);
      // If translation key is returned (not found), return the role itself
      return translated === roleKey ? role : translated;
    },
    [t]
  );

  // Get avatar state for an agent
  const getAvatarState = useCallback(
    (agentId: string): AgentAvatarState => {
      return (
        avatars[agentId] || {
          url: null,
          uploading: false,
          success: false,
          error: null,
        }
      );
    },
    [avatars]
  );

  // Handle avatar upload dialog
  const handleUploadAvatar = useCallback(
    async (agentId: string, agentName: string) => {
      // Set uploading state
      setAvatars((prev) => ({
        ...prev,
        [agentId]: {
          ...getAvatarState(agentId),
          uploading: true,
          error: null,
        },
      }));

      try {
        const result = await AvatarUploadDialog.show({ agentId, agentName });

        if (result && result.url) {
          // Update avatar URL on success
          setAvatars((prev) => ({
            ...prev,
            [agentId]: {
              url: result.url,
              uploading: false,
              success: true,
              error: null,
            },
          }));
          setUploadSuccess(agentName);
          setTimeout(() => setUploadSuccess(null), 3000);
        } else {
          // User cancelled
          setAvatars((prev) => ({
            ...prev,
            [agentId]: {
              ...getAvatarState(agentId),
              uploading: false,
            },
          }));
        }
      } catch (err) {
        // Error or cancelled
        setAvatars((prev) => ({
          ...prev,
          [agentId]: {
            ...getAvatarState(agentId),
            uploading: false,
            error:
              err instanceof Error
                ? err.message
                : t('settings.agentsConfiguration.upload.error'),
          },
        }));
      }
    },
    [getAvatarState, t]
  );

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-8 w-8 animate-spin" />
        <span className="ml-2">{t('settings.agentsConfiguration.loading')}</span>
      </div>
    );
  }

  if (isError) {
    return (
      <div className="py-8">
        <Alert variant="destructive">
          <AlertDescription>
            {error instanceof Error ? error.message : String(error)}
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Success Alert */}
      {uploadSuccess && (
        <Alert variant="success">
          <CheckCircle2 className="h-4 w-4" />
          <AlertDescription className="font-medium">
            {t('settings.agentsConfiguration.upload.success')}
          </AlertDescription>
        </Alert>
      )}

      {/* Header */}
      <div>
        <h2 className="text-2xl font-semibold">
          {t('settings.agentsConfiguration.title')}
        </h2>
        <p className="text-muted-foreground mt-1">
          {t('settings.agentsConfiguration.description')}
        </p>
      </div>

      {/* Claude Orchestrator Card - Special styling */}
      <Card className="border-primary/20 bg-gradient-to-r from-primary/5 to-primary/10">
        <CardHeader>
          <div className="flex items-center gap-4">
            <UserAvatar
              firstName="Claude"
              lastName=""
              username="claude"
              imageUrl={getAvatarState('CLAUDE').url}
              className="h-16 w-16 text-lg"
            />
            <div className="flex-1">
              <CardTitle className="flex items-center gap-2">
                {t('settings.agentsConfiguration.claude')}
                <span className="text-xs bg-primary/20 text-primary px-2 py-0.5 rounded-full">
                  {t('settings.agentsConfiguration.orchestrator')}
                </span>
              </CardTitle>
              <CardDescription>
                Multi-agent orchestration coordinator
              </CardDescription>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => handleUploadAvatar('CLAUDE', 'Claude')}
              disabled={getAvatarState('CLAUDE').uploading}
            >
              {getAvatarState('CLAUDE').uploading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t('settings.agentsConfiguration.agent.uploading')}
                </>
              ) : (
                <>
                  <Upload className="mr-2 h-4 w-4" />
                  {t('settings.agentsConfiguration.agent.uploadButton')}
                </>
              )}
            </Button>
          </div>
        </CardHeader>
      </Card>

      {/* Agent Grid */}
      {agents.length === 0 ? (
        <Card>
          <CardContent className="py-8 text-center text-muted-foreground">
            {t('settings.agentsConfiguration.noAgents')}
          </CardContent>
        </Card>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {agents.map((agent) => {
            const avatarState = getAvatarState(agent.id);
            return (
              <Card key={agent.id}>
                <CardContent className="pt-6">
                  <div className="flex flex-col items-center text-center gap-3">
                    <UserAvatar
                      firstName={agent.name.charAt(0)}
                      lastName={agent.name.charAt(1) || ''}
                      username={agent.name}
                      imageUrl={avatarState.url}
                      className="h-16 w-16 text-lg"
                    />
                    <div>
                      <h3 className="font-semibold">{agent.name}</h3>
                      <p className="text-sm text-muted-foreground">
                        {getAgentRoleDisplayName(agent.role)}
                      </p>
                    </div>
                    {avatarState.error && (
                      <p className="text-xs text-destructive">
                        {avatarState.error}
                      </p>
                    )}
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleUploadAvatar(agent.id, agent.name)}
                      disabled={avatarState.uploading}
                      className="w-full"
                    >
                      {avatarState.uploading ? (
                        <>
                          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                          {t('settings.agentsConfiguration.agent.uploading')}
                        </>
                      ) : (
                        <>
                          <Upload className="mr-2 h-4 w-4" />
                          {t('settings.agentsConfiguration.agent.uploadButton')}
                        </>
                      )}
                    </Button>
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}
