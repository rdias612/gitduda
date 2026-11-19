import type { StatusEntry } from '../types/git.types.ts';

export interface DefaultCommitMessage {
  summary: string;
  description: string;
}

export function formatStagedFileLine(entry: Pick<StatusEntry, 'path' | 'status'>): string {
  switch (entry.status) {
    case 'new':
    case 'untracked':
      return `add file ${entry.path}`;
    case 'modified':
      return `modified file ${entry.path}`;
    case 'deleted':
      return `deleted file ${entry.path}`;
    case 'renamed':
      return `renamed file ${entry.path}`;
    case 'copied':
      return `copied file ${entry.path}`;
    case 'typechange':
      return `changed file ${entry.path}`;
    default:
      return `update file ${entry.path}`;
  }
}

export function buildDefaultCommitMessage(
  stagedFiles: ReadonlyArray<Pick<StatusEntry, 'path' | 'status'>>
): DefaultCommitMessage {
  if (stagedFiles.length === 0) {
    return { summary: '', description: '' };
  }

  if (stagedFiles.length === 1) {
    return {
      summary: formatStagedFileLine(stagedFiles[0]),
      description: '',
    };
  }

  return {
    summary: 'multiple changes',
    description: stagedFiles.map(formatStagedFileLine).join('\n'),
  };
}
