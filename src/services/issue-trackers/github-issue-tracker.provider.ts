import * as git from '../git.service.ts';
import { unifiedProfileStore } from '../../stores/unified-profile.store.ts';
import { loadUnifiedProfiles } from '../unified-profile.service.ts';
import type { IssueTrackerProvider } from '../issue-tracker.service.ts';

export const githubIssueTrackerProvider: IssueTrackerProvider = {
  id: 'github',
  displayName: 'GitHub',
  async detectTarget(path) {
    const result = await git.detectGitHubRepo(path);
    if (!result.success) throw new Error(result.error?.message);
    if (!result.data) return null;
    const { owner, repo } = result.data;
    return { providerId: this.id, providerName: this.displayName, repoLabel: `${owner}/${repo}`, metadata: { owner, repo } };
  },
  async getToken() {
    if (!unifiedProfileStore.getState().accounts.length) await loadUnifiedProfiles();
    const result = await git.getGitHubToken();
    if (!result.success) throw new Error(result.error?.message);
    return result.data ?? null;
  },
  async listOpenIssues(target, token, limit = 50) {
    const { owner, repo } = target.metadata as { owner: string; repo: string };
    const result = await git.listIssues(owner, repo, 'open', undefined, limit, 1, token);
    if (!result.success || !result.data) return { success: false, error: result.error };
    return { success: true, data: result.data.issues.slice(0, limit).map(issue => ({
      id: String(issue.number), key: `#${issue.number}`, title: issue.title, state: issue.state,
      url: issue.htmlUrl, labels: issue.labels.map(label => label.name),
    })) };
  },
  formatReference(issue, relation = 'refs') {
    return `${relation === 'closes' ? 'Closes' : 'Refs'} ${issue.key}`;
  },
};
