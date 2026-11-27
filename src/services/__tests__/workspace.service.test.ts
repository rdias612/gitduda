import { expect } from '@open-wc/testing';

// Mock Tauri API
type MockInvoke = (command: string, args?: unknown) => Promise<unknown>;
let mockInvoke: MockInvoke = () => Promise.resolve(null);
let lastInvokedCommand: string | null = null;
let lastInvokedArgs: unknown = null;

(globalThis as unknown as { __TAURI_INTERNALS__: { invoke: MockInvoke } }).__TAURI_INTERNALS__ = {
  invoke: (command: string, args?: unknown) => {
    lastInvokedCommand = command;
    lastInvokedArgs = args;
    return mockInvoke(command, args);
  },
};

import {
  getWorkspaces,
  getWorkspace,
  saveWorkspace,
  deleteWorkspace,
  addRepositoryToWorkspace,
  removeRepositoryFromWorkspace,
  updateWorkspaceLastOpened,
  validateWorkspaceRepositories,
  openWorkspace,
} from '../workspace.service.ts';
import { repositoryStore } from '../../stores/repository.store.ts';
import { workspaceStore } from '../../stores/workspace.store.ts';
import type { Repository, Workspace, WorkspaceRepoStatus } from '../../types/git.types.ts';

const mockWorkspace: Workspace = {
  id: 'ws-1',
  name: 'Test Workspace',
  description: 'A test workspace',
  color: '#4fc3f7',
  repositories: [
    { path: '/repo/one', name: 'one' },
    { path: '/repo/two', name: 'two' },
  ],
  createdAt: '2025-01-01T00:00:00Z',
  lastOpened: null,
};

describe('workspace.service', () => {
  beforeEach(() => {
    lastInvokedCommand = null;
    lastInvokedArgs = null;
  });

  describe('getWorkspaces', () => {
    it('invokes get_workspaces command', async () => {
      mockInvoke = () => Promise.resolve([mockWorkspace]);

      const result = await getWorkspaces();
      expect(lastInvokedCommand).to.equal('get_workspaces');
      expect(result.success).to.be.true;
      expect(result.data?.length).to.equal(1);
    });

    it('returns empty array when no workspaces', async () => {
      mockInvoke = () => Promise.resolve([]);

      const result = await getWorkspaces();
      expect(result.success).to.be.true;
      expect(result.data).to.deep.equal([]);
    });
  });

  describe('getWorkspace', () => {
    it('invokes get_workspace with workspaceId', async () => {
      mockInvoke = () => Promise.resolve(mockWorkspace);

      const result = await getWorkspace('ws-1');
      expect(lastInvokedCommand).to.equal('get_workspace');
      expect((lastInvokedArgs as Record<string, unknown>).workspaceId).to.equal('ws-1');
      expect(result.success).to.be.true;
      expect(result.data?.name).to.equal('Test Workspace');
    });

    it('handles workspace not found', async () => {
      mockInvoke = () => Promise.reject({ code: 'OPERATION_FAILED', message: 'Workspace not found' });

      const result = await getWorkspace('nonexistent');
      expect(result.success).to.be.false;
    });
  });

  describe('saveWorkspace', () => {
    it('invokes save_workspace with workspace object', async () => {
      mockInvoke = () => Promise.resolve(mockWorkspace);

      const result = await saveWorkspace(mockWorkspace);
      expect(lastInvokedCommand).to.equal('save_workspace');
      expect((lastInvokedArgs as Record<string, unknown>).workspace).to.deep.equal(mockWorkspace);
      expect(result.success).to.be.true;
    });

    it('handles save error', async () => {
      mockInvoke = () => Promise.reject({ code: 'OPERATION_FAILED', message: 'Failed to save' });

      const result = await saveWorkspace(mockWorkspace);
      expect(result.success).to.be.false;
    });
  });

  describe('deleteWorkspace', () => {
    it('invokes delete_workspace with workspaceId', async () => {
      mockInvoke = () => Promise.resolve(null);

      const result = await deleteWorkspace('ws-1');
      expect(lastInvokedCommand).to.equal('delete_workspace');
      expect((lastInvokedArgs as Record<string, unknown>).workspaceId).to.equal('ws-1');
      expect(result.success).to.be.true;
    });
  });

  describe('addRepositoryToWorkspace', () => {
    it('invokes add_repository_to_workspace with camelCase args', async () => {
      mockInvoke = () => Promise.resolve(mockWorkspace);

      const result = await addRepositoryToWorkspace('ws-1', '/new/repo', 'repo');
      expect(lastInvokedCommand).to.equal('add_repository_to_workspace');
      const args = lastInvokedArgs as Record<string, unknown>;
      expect(args.workspaceId).to.equal('ws-1');
      expect(args.path).to.equal('/new/repo');
      expect(args.name).to.equal('repo');
      expect(result.success).to.be.true;
    });
  });

  describe('removeRepositoryFromWorkspace', () => {
    it('invokes remove_repository_from_workspace with camelCase args', async () => {
      mockInvoke = () => Promise.resolve(mockWorkspace);

      const result = await removeRepositoryFromWorkspace('ws-1', '/repo/one');
      expect(lastInvokedCommand).to.equal('remove_repository_from_workspace');
      const args = lastInvokedArgs as Record<string, unknown>;
      expect(args.workspaceId).to.equal('ws-1');
      expect(args.path).to.equal('/repo/one');
      expect(result.success).to.be.true;
    });
  });

  describe('updateWorkspaceLastOpened', () => {
    it('invokes update_workspace_last_opened with workspaceId', async () => {
      mockInvoke = () => Promise.resolve(null);

      const result = await updateWorkspaceLastOpened('ws-1');
      expect(lastInvokedCommand).to.equal('update_workspace_last_opened');
      expect((lastInvokedArgs as Record<string, unknown>).workspaceId).to.equal('ws-1');
      expect(result.success).to.be.true;
    });
  });

  describe('switching workspaces', () => {
    const sharedPath = 'C:\\Repos\\Shared';
    const oldPath = '/repo/old';

    function makeRepository(path: string): Repository {
      return {
        path,
        name: path.split(/[\\/]/).at(-1) || path,
        isValid: true,
        isBare: false,
        headRef: 'main',
        detachedHeadOid: null,
        state: 'clean',
        isShallow: false,
        isPartialClone: false,
        cloneFilter: null,
      };
    }

    beforeEach(() => {
      repositoryStore.getState().reset();
      workspaceStore.getState().setActiveWorkspaceId('old');
      repositoryStore.getState().addRepository(makeRepository(oldPath));
      repositoryStore.getState().addRepository(makeRepository(sharedPath));
    });

    afterEach(() => {
      repositoryStore.getState().reset();
      workspaceStore.getState().reset();
    });

    it('keeps the shared tab and persists only the destination tabs', async () => {
      const active = repositoryStore.getState().getActiveRepository();
      mockInvoke = async (command, args) =>
        command === 'open_repository' ? makeRepository((args as { path: string }).path) : null;

      await openWorkspace({
        ...mockWorkspace,
        repositories: [{ path: 'c:/repos/shared', name: 'Shared' }, ...mockWorkspace.repositories],
      });

      const store = repositoryStore.getState();
      expect(store.getActiveRepository()).to.equal(active);
      expect(store.openRepositories.map(({ repository }) => repository.path)).to.deep.equal([
        sharedPath,
        '/repo/one',
        '/repo/two',
      ]);
      expect(store.persistedOpenRepos.map(({ path }) => path)).to.deep.equal([
        sharedPath,
        '/repo/one',
        '/repo/two',
      ]);
      expect(store.recentRepositories.map(({ path }) => path)).to.include(oldPath);
      expect(workspaceStore.getState().activeWorkspaceId).to.equal(mockWorkspace.id);
    });

    it('keeps the current workspace and tabs if every destination repository fails', async () => {
      const current = repositoryStore.getState();
      mockInvoke = async () => {
        throw new Error('Repository not found');
      };

      await openWorkspace(mockWorkspace);

      const store = repositoryStore.getState();
      expect(store.openRepositories).to.equal(current.openRepositories);
      expect(store.getActiveRepository()).to.equal(current.getActiveRepository());
      expect(workspaceStore.getState().activeWorkspaceId).to.equal('old');
      expect(workspaceStore.getState().isLoading).to.be.false;
    });
  });

  describe('validateWorkspaceRepositories', () => {
    it('invokes validate_workspace_repositories with workspaceId', async () => {
      const mockStatuses: WorkspaceRepoStatus[] = [
        {
          path: '/repo/one',
          name: 'one',
          exists: true,
          isValidRepo: true,
          changedFilesCount: 3,
          currentBranch: 'main',
          isDetached: false,
          ahead: 1,
          behind: 0,
        },
        {
          path: '/repo/two',
          name: 'two',
          exists: false,
          isValidRepo: false,
          changedFilesCount: 0,
          currentBranch: null,
          isDetached: false,
          ahead: 0,
          behind: 0,
        },
        {
          path: '/repo/three',
          name: 'three',
          exists: true,
          isValidRepo: true,
          changedFilesCount: 0,
          currentBranch: null,
          isDetached: true,
          ahead: 0,
          behind: 0,
        },
      ];
      mockInvoke = () => Promise.resolve(mockStatuses);

      const result = await validateWorkspaceRepositories('ws-1');
      expect(lastInvokedCommand).to.equal('validate_workspace_repositories');
      expect((lastInvokedArgs as Record<string, unknown>).workspaceId).to.equal('ws-1');
      expect(result.success).to.be.true;
      expect(result.data?.length).to.equal(3);
      expect(result.data?.[0].changedFilesCount).to.equal(3);
      expect(result.data?.[1].exists).to.be.false;
      // isDetached crosses the IPC boundary in camelCase, as Tauri renames it
      expect(result.data?.[0].isDetached).to.be.false;
      expect(result.data?.[2].isDetached).to.be.true;
      expect(result.data?.[2].currentBranch).to.be.null;
    });
  });
});
