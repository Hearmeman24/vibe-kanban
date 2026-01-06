import {
  DataWithScrollModifier,
  ScrollModifier,
  VirtuosoMessageList,
  VirtuosoMessageListLicense,
  VirtuosoMessageListMethods,
  VirtuosoMessageListProps,
} from '@virtuoso.dev/message-list';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import DisplayConversationEntry from '../NormalizedConversation/DisplayConversationEntry';
import { useEntries } from '@/contexts/EntriesContext';
import {
  AddEntryType,
  PatchTypeWithKey,
  useConversationHistory,
} from '@/hooks/useConversationHistory';
import { Bot, Loader2 } from 'lucide-react';
import { AgentMetadataEntry, TaskWithAttemptStatus } from 'shared/types';
import type { WorkspaceWithSession } from '@/types/attempt';
import { ApprovalFormProvider } from '@/contexts/ApprovalFormContext';

interface VirtualizedListProps {
  attempt: WorkspaceWithSession;
  task?: TaskWithAttemptStatus;
}

interface MessageListContext {
  attempt: WorkspaceWithSession;
  task?: TaskWithAttemptStatus;
}

const INITIAL_TOP_ITEM = { index: 'LAST' as const, align: 'end' as const };

const InitialDataScrollModifier: ScrollModifier = {
  type: 'item-location',
  location: INITIAL_TOP_ITEM,
  purgeItemSizes: true,
};

const AutoScrollToBottom: ScrollModifier = {
  type: 'auto-scroll-to-bottom',
  autoScroll: 'smooth',
};

const ItemContent: VirtuosoMessageListProps<
  PatchTypeWithKey,
  MessageListContext
>['ItemContent'] = ({ data, context }) => {
  const attempt = context?.attempt;
  const task = context?.task;

  if (data.type === 'STDOUT') {
    return <p>{data.content}</p>;
  }
  if (data.type === 'STDERR') {
    return <p>{data.content}</p>;
  }
  if (data.type === 'NORMALIZED_ENTRY' && attempt) {
    return (
      <DisplayConversationEntry
        expansionKey={data.patchKey}
        entry={data.content}
        executionProcessId={data.executionProcessId}
        taskAttempt={attempt}
        task={task}
      />
    );
  }

  return null;
};

const computeItemKey: VirtuosoMessageListProps<
  PatchTypeWithKey,
  MessageListContext
>['computeItemKey'] = ({ data }) => `l-${data.patchKey}`;

const VirtualizedList = ({ attempt, task }: VirtualizedListProps) => {
  const { t } = useTranslation('tasks');
  const [channelData, setChannelData] =
    useState<DataWithScrollModifier<PatchTypeWithKey> | null>(null);
  const [loading, setLoading] = useState(true);
  const { setEntries, reset } = useEntries();

  // Check if this is an ORCHESTRATOR_MANAGED workspace (no container_ref)
  const isOrchestratorManaged = attempt.container_ref === null;

  // Parse agent metadata from task if available
  const agentNames = useMemo(() => {
    if (!task?.agent_metadata) return [];
    try {
      const metadata: AgentMetadataEntry[] = JSON.parse(task.agent_metadata);
      // Get unique agent names
      const uniqueNames = [...new Set(metadata.map((entry) => entry.agent_name))];
      return uniqueNames;
    } catch {
      return [];
    }
  }, [task?.agent_metadata]);

  useEffect(() => {
    setLoading(true);
    setChannelData(null);
    reset();
  }, [attempt.id, reset]);

  const onEntriesUpdated = (
    newEntries: PatchTypeWithKey[],
    addType: AddEntryType,
    newLoading: boolean
  ) => {
    let scrollModifier: ScrollModifier = InitialDataScrollModifier;

    if (addType === 'running' && !loading) {
      scrollModifier = AutoScrollToBottom;
    }

    setChannelData({ data: newEntries, scrollModifier });
    setEntries(newEntries);

    if (loading) {
      setLoading(newLoading);
    }
  };

  // Only use conversation history for non-orchestrator-managed workspaces
  useConversationHistory({
    attempt,
    onEntriesUpdated: isOrchestratorManaged ? () => {} : onEntriesUpdated,
  });

  const messageListRef = useRef<VirtuosoMessageListMethods | null>(null);
  const messageListContext = useMemo(
    () => ({ attempt, task }),
    [attempt, task]
  );

  // For ORCHESTRATOR_MANAGED workspaces, show agent info instead of conversation history
  if (isOrchestratorManaged) {
    return (
      <div className="flex-1 flex flex-col gap-3 justify-center items-center p-8 text-center">
        <Bot className="h-12 w-12 text-muted-foreground" />
        <p className="text-muted-foreground text-lg">
          {agentNames.length > 0
            ? t('orchestrator.managedBy', { agents: agentNames.join(', ') })
            : t('orchestrator.noAgent')}
        </p>
        <p className="text-muted-foreground/70 text-sm max-w-md">
          {t('orchestrator.description')}
        </p>
      </div>
    );
  }

  return (
    <ApprovalFormProvider>
      <VirtuosoMessageListLicense
        licenseKey={import.meta.env.VITE_PUBLIC_REACT_VIRTUOSO_LICENSE_KEY}
      >
        <VirtuosoMessageList<PatchTypeWithKey, MessageListContext>
          ref={messageListRef}
          className="flex-1"
          data={channelData}
          initialLocation={INITIAL_TOP_ITEM}
          context={messageListContext}
          computeItemKey={computeItemKey}
          ItemContent={ItemContent}
          Header={() => <div className="h-2"></div>}
          Footer={() => <div className="h-2"></div>}
        />
      </VirtuosoMessageListLicense>
      {loading && (
        <div className="float-left top-0 left-0 w-full h-full bg-primary flex flex-col gap-2 justify-center items-center">
          <Loader2 className="h-8 w-8 animate-spin" />
          <p>Loading History</p>
        </div>
      )}
    </ApprovalFormProvider>
  );
};

export default VirtualizedList;
