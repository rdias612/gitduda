import { registerIssueTrackerProvider } from '../issue-tracker.service.ts';
import { githubIssueTrackerProvider } from './github-issue-tracker.provider.ts';

registerIssueTrackerProvider(githubIssueTrackerProvider);
