import { settingsStore } from '../stores/settings.store.ts';
import type { CommandResult } from '../types/api.types.ts';

export type IssueReferenceRelation = 'refs' | 'closes';
export interface IssueTrackerTarget {
  providerId: string;
  providerName: string;
  repoLabel: string;
  metadata: unknown;
}
export interface TrackedIssue {
  id: string;
  key: string;
  title: string;
  state: string;
  url: string;
  labels?: string[];
}
export type IssueListState =
  | { status: 'ready'; target: IssueTrackerTarget; issues: TrackedIssue[] }
  | { status: 'no-provider' | 'offline' | 'blocked' | 'unauthenticated' }
  | { status: 'error'; message: string };

export interface IssueTrackerProvider {
  readonly id: string;
  readonly displayName: string;
  detectTarget(repoPath: string): Promise<IssueTrackerTarget | null>;
  getToken(target: IssueTrackerTarget): Promise<string | null>;
  listOpenIssues(target: IssueTrackerTarget, token: string | null, limit?: number): Promise<CommandResult<TrackedIssue[]>>;
  formatReference(issue: TrackedIssue, relation?: IssueReferenceRelation): string;
}

const providers = new Map<string, IssueTrackerProvider>();
const detections = new Map<string, Promise<IssueTrackerTarget | null>>();
const lists = new Map<string, { token: string; result: Promise<IssueListState> }>();

export function invalidateIssueTrackerCache(path?: string) {
  if (path === undefined) {
    detections.clear();
    lists.clear();
  } else {
    detections.delete(path);
    lists.delete(path);
  }
}

settingsStore.subscribe((next, previous) => {
  if (next.offlineMode !== previous.offlineMode || next.remoteAllowlist !== previous.remoteAllowlist) {
    invalidateIssueTrackerCache();
  }
});

export function registerIssueTrackerProvider(provider: IssueTrackerProvider) {
  providers.set(provider.id, provider);
  invalidateIssueTrackerCache();
}

export function unregisterIssueTrackerProvider(id: string) {
  providers.delete(id);
  invalidateIssueTrackerCache();
}

export function clearIssueTrackerProviders() {
  providers.clear();
  invalidateIssueTrackerCache();
}

export async function detectIssueTracker(path: string): Promise<IssueTrackerTarget | null> {
  const cached = detections.get(path);
  if (cached) return cached;
  const detection = (async () => {
    for (const provider of providers.values()) {
      const target = await provider.detectTarget(path);
      if (target) return target;
    }
    return null;
  })();
  detections.set(path, detection);
  try {
    return await detection;
  } catch (error) {
    if (detections.get(path) === detection) detections.delete(path);
    throw error;
  }
}

export async function listOpenIssues(path: string): Promise<IssueListState> {
  if (settingsStore.getState().offlineMode) return { status: 'offline' };
  try {
    const detection = detectIssueTracker(path);
    const generation = detections.get(path);
    const target = await detection;
    const provider = target && providers.get(target.providerId);
    if (!target || !provider) return { status: 'no-provider' };
    const token = await provider.getToken(target);
    if (!token) return { status: 'unauthenticated' };
    const cached = lists.get(path);
    if (cached?.token === token) return await cached.result;
    const result = provider.listOpenIssues(target, token).then((response): IssueListState => {
      if (response.success && response.data) return { status: 'ready', target, issues: response.data };
      if (response.error?.code === 'BLOCKED') {
        return { status: settingsStore.getState().offlineMode ? 'offline' : 'blocked' };
      }
      return { status: 'error', message: response.error?.message ?? '' };
    });
    if (generation === detections.get(path)) lists.set(path, { token, result });
    try {
      const state = await result;
      if (state.status !== 'ready' && lists.get(path)?.result === result) lists.delete(path);
      return state;
    } catch (error) {
      if (lists.get(path)?.result === result) lists.delete(path);
      throw error;
    }
  } catch (error) {
    return { status: 'error', message: error instanceof Error ? error.message : String(error) };
  }
}

export function searchIssues(issues: TrackedIssue[], query: string) {
  const needle = query.trim().replace(/^#/, '').toLowerCase();
  return issues.filter(issue => [issue.key, issue.id, issue.title].some(value => value.toLowerCase().includes(needle)));
}

export function formatIssueReference(target: IssueTrackerTarget, issue: TrackedIssue, relation: IssueReferenceRelation = 'refs') {
  return providers.get(target.providerId)?.formatReference(issue, relation);
}
