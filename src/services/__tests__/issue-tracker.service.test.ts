import { expect } from '@open-wc/testing';
import { detectIssueTracker, formatIssueReference, listOpenIssues, registerIssueTrackerProvider,
  invalidateIssueTrackerCache, searchIssues, clearIssueTrackerProviders, type IssueTrackerProvider } from '../issue-tracker.service.ts';
import { githubIssueTrackerProvider } from '../issue-trackers/github-issue-tracker.provider.ts';
import { settingsStore } from '../../stores/settings.store.ts';
import { appendIssueReference, detectIssueTrigger, stripIssueTrigger } from '../../utils/issue-reference.ts';

describe('issue references', () => {
  it('detects only standalone triggers up to the caret', () => {
    expect(detectIssueTrigger('C#', 2)).to.equal(null);
    expect(detectIssueTrigger('a #two words', 12)).to.equal(null);
    const text = 'first\n\n#query second';
    const match = detectIssueTrigger(text, 13)!;
    expect(match).to.deep.equal({ start: 7, end: 13, query: 'query' });
    expect(stripIssueTrigger(text, match)).to.equal('first\n\nsecond');
  });
  it('groups references after the body and deduplicates entire lines', () => {
    const first = appendIssueReference('Body', 'Refs #23');
    expect(first).to.equal('Body\n\nRefs #23');
    expect(appendIssueReference(first, 'refs #23')).to.equal(first);
    expect(appendIssueReference(first, 'Closes PROJ-2')).to.equal(first + '\nCloses PROJ-2');
  });
});

describe('issue trackers', () => {
  afterEach(() => {
    clearIssueTrackerProviders();
    settingsStore.getState().setOfflineMode(false);
    settingsStore.getState().setRemoteAllowlist([]);
  });

  it('shares detection and list requests per repo and dispatches the reference to its provider', async () => {
    let detects = 0, lists = 0;
    const issue = { id: '123', key: 'PROJ-123', title: 'Review', url: '', state: 'open' };
    const adapter: IssueTrackerProvider = {
      id: 'jira', displayName: 'Jira',
      detectTarget: async repoLabel => {
        detects++;
        return { providerId: 'jira', providerName: 'Jira', repoLabel, metadata: null };
      },
      getToken: async () => 'credential',
      listOpenIssues: async () => { lists++; return { success: true, data: [issue] }; },
      formatReference: (item, relation) => (relation === 'closes' ? 'Closes ' : 'Refs ') + item.key,
    };
    registerIssueTrackerProvider({ ...adapter, id: 'unrelated', detectTarget: async () => null });
    registerIssueTrackerProvider(adapter);
    const [state] = await Promise.all([listOpenIssues('a'), listOpenIssues('a')]);
    expect(state.status).to.equal('ready');
    expect([detects, lists]).to.deep.equal([1, 1]);
    const target = (await detectIssueTracker('a'))!;
    expect(formatIssueReference(target, issue, 'closes')).to.equal('Closes PROJ-123');
    expect(formatIssueReference(target, issue)).to.equal('Refs PROJ-123');
    for (const query of ['#123', 'proj-', 'REVIEW']) expect(searchIssues([issue], query)).to.deep.equal([issue]);
    await listOpenIssues('b');
    invalidateIssueTrackerCache('a');
    await listOpenIssues('a');
    expect([detects, lists]).to.deep.equal([3, 3]);
  });

  it('maps the IPC result and retries after offline, blocked, missing credentials and API errors', async () => {
    let token: string | null = null;
    let error = true;
    let calls = 0;
    const globals = globalThis as Record<string, unknown>;
    const previous = globals.__TAURI_INTERNALS__;
    globals.__TAURI_INTERNALS__ = { invoke: async (cmd: string, args: Record<string, unknown>) => {
      if (cmd === 'get_keyring_token') return token;
      if (cmd === 'detect_github_repo') return { owner: 'org', repo: 'repo' };
      if (cmd === 'list_issues') {
        calls++;
        expect(args).to.include({ owner: 'org', repo: 'repo', state: 'open', perPage: 50, page: 1, token });
        if (error) throw new Error('Request failed');
        return { issues: [{ number: 23, title: 'Editor', state: 'open', htmlUrl: 'issue-url', labels: [{ name: 'ui' }] }] };
      }
      return null;
    } };
    try {
      registerIssueTrackerProvider(githubIssueTrackerProvider);
      settingsStore.getState().setOfflineMode(true);
      expect((await listOpenIssues('a')).status).to.equal('offline');
      settingsStore.getState().setOfflineMode(false);
      expect((await listOpenIssues('a')).status).to.equal('unauthenticated');
      token = 'token';
      settingsStore.getState().setRemoteAllowlist(['example.org']);
      expect((await listOpenIssues('a')).status).to.equal('blocked');
      expect(calls).to.equal(0);
      settingsStore.getState().setRemoteAllowlist([]);
      expect((await listOpenIssues('a')).status).to.equal('error');
      error = false;
      const state = await listOpenIssues('a');
      if (state.status !== 'ready') throw new Error(state.status);
      expect(state.issues[0]).to.deep.equal({ id: '23', key: '#23', title: 'Editor', state: 'open', url: 'issue-url', labels: ['ui'] });
      expect(formatIssueReference(state.target, state.issues[0])).to.equal('Refs #23');
    } finally {
      globals.__TAURI_INTERNALS__ = previous;
    }
  });
});
