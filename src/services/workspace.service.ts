/**
 * Workspace Service
 * Manages multi-repository workspaces via Tauri commands
 */

import { msg, str } from '@lit/localize';
import { repositoryStore } from '../stores/repository.store.ts';
import { workspaceStore } from '../stores/workspace.store.ts';
import { invokeCommand } from './tauri-api.ts';
import { openRepository } from './git/repository.ts';
import { showToast } from './notification.service.ts';
import type {
  Repository,
  Workspace,
  WorkspaceRepoStatus,
  WorkspaceSearchResponse,
} from '../types/git.types.ts';
import type { CommandResult } from '../types/api.types.ts';

export async function getWorkspaces(): Promise<CommandResult<Workspace[]>> {
  return invokeCommand<Workspace[]>('get_workspaces');
}

export async function getWorkspace(workspaceId: string): Promise<CommandResult<Workspace>> {
  return invokeCommand<Workspace>('get_workspace', { workspaceId });
}

export async function saveWorkspace(workspace: Workspace): Promise<CommandResult<Workspace>> {
  const result = await invokeCommand<Workspace>('save_workspace', { workspace });
  if (result.success) {
    showToast('Workspace saved', 'success');
  }
  return result;
}

export async function deleteWorkspace(workspaceId: string): Promise<CommandResult<void>> {
  // Deliberately silent: the sole caller (gn-workspace-manager-dialog) names
  // which workspace went, and toasting here too stacked two success messages
  // for one click.
  return invokeCommand<void>('delete_workspace', { workspaceId });
}

export async function addRepositoryToWorkspace(
  workspaceId: string,
  path: string,
  name: string,
): Promise<CommandResult<Workspace>> {
  return invokeCommand<Workspace>('add_repository_to_workspace', {
    workspaceId,
    path,
    name,
  });
}

export async function removeRepositoryFromWorkspace(
  workspaceId: string,
  path: string,
): Promise<CommandResult<Workspace>> {
  return invokeCommand<Workspace>('remove_repository_from_workspace', {
    workspaceId,
    path,
  });
}

export async function updateWorkspaceLastOpened(workspaceId: string): Promise<CommandResult<void>> {
  return invokeCommand<void>('update_workspace_last_opened', { workspaceId });
}

export async function openWorkspace(
  workspace: Workspace,
  repoStatuses?: ReadonlyMap<string, WorkspaceRepoStatus>,
): Promise<void> {
  const workspaceState = workspaceStore.getState();
  if (workspaceState.isLoading) return;
  const switchingWorkspace =
    workspaceState.activeWorkspaceId !== null && workspaceState.activeWorkspaceId !== workspace.id;
  workspaceState.setLoading(true);

  try {
    const openedRepositories: Repository[] = [];
    let failedCount = 0;
    for (const repo of workspace.repositories) {
      const status = repoStatuses?.get(repo.path);
      if (status && (!status.exists || !status.isValidRepo)) {
        failedCount++;
        continue;
      }

      const result = await openRepository({ path: repo.path });
      if (result.success && result.data) {
        openedRepositories.push(result.data);
      } else {
        failedCount++;
      }
    }

    if (failedCount > 0 && openedRepositories.length === 0) {
      showToast(
        msg(str`Could not open workspace: ${workspace.name} — no repository could be opened`),
        'error',
      );
      return;
    }

    const store = repositoryStore.getState();
    if (switchingWorkspace) {
      store.replaceRepositories(openedRepositories);
    } else {
      // Activate once after the batch to avoid starting index builds for every repo.
      for (const repo of openedRepositories) {
        store.addRepository(repo, { activate: false });
      }
      const lastOpened = openedRepositories.at(-1);
      if (lastOpened) store.setActiveByPath(lastOpened.path);
    }
    workspaceState.setActiveWorkspaceId(workspace.id);
    await updateWorkspaceLastOpened(workspace.id);

    if (failedCount === 0) {
      showToast(msg(str`Opened workspace: ${workspace.name}`), 'success');
    } else {
      showToast(
        msg(
          str`Opened workspace: ${workspace.name} (${failedCount} of ${workspace.repositories.length} repositories failed to open)`,
        ),
        'warning',
      );
    }
  } finally {
    workspaceState.setLoading(false);
  }
}

export async function validateWorkspaceRepositories(
  workspaceId: string,
): Promise<CommandResult<WorkspaceRepoStatus[]>> {
  return invokeCommand<WorkspaceRepoStatus[]>('validate_workspace_repositories', {
    workspaceId,
  });
}

export async function searchWorkspace(
  workspaceId: string,
  query: string,
  caseSensitive?: boolean,
  regex?: boolean,
  filePattern?: string,
  maxResults?: number,
): Promise<CommandResult<WorkspaceSearchResponse>> {
  return invokeCommand<WorkspaceSearchResponse>('search_workspace', {
    workspaceId,
    query,
    caseSensitive,
    regex,
    filePattern,
    maxResults,
  });
}

export async function exportWorkspace(
  workspaceId: string,
): Promise<CommandResult<string>> {
  return invokeCommand<string>('export_workspace', { workspaceId });
}

export async function importWorkspace(
  jsonData: string,
): Promise<CommandResult<Workspace>> {
  return invokeCommand<Workspace>('import_workspace', { jsonData });
}
