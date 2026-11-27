import { expect, test } from '@playwright/test';
import { setupOpenRepository } from '../fixtures/tauri-mock';
import { emitBackendEvent, findCommand, injectCommandMock, startCommandCapture } from '../fixtures/test-helpers';
import type { SyncRefChange, SyncRefsPreview } from '../../src/types/git.types';

const remotes = [
  { name: 'origin', url: '/path/to/origin', pushUrl: null },
  { name: 'upstream', url: '/path/to/upstream', pushUrl: null },
];

const localOid = 'a'.repeat(40);
const remoteOid = 'b'.repeat(40);

const change = (name: string, fields: Partial<SyncRefChange> = {}): SyncRefChange => ({
  name,
  localOid,
  remoteOid: null,
  outgoingCommits: 0,
  worktree: null,
  protected: false,
  ...fields,
});

const empty: SyncRefsPreview = {
  revision: 'first',
  remoteCount: 0,
  newRefs: [],
  deletions: [],
  overwrites: [],
  currentBranch: null,
  currentBranchDeleted: false,
  workingChanges: false,
  worktreeBusy: false,
  replacements: [],
};

test.beforeEach(async ({ page }) => {
  await setupOpenRepository(page);
  await startCommandCapture(page);
});

test('tag sync selects the sole remote and cancel leaves the preview unapplied', async ({ page }) => {
  await injectCommandMock(page, {
    get_remotes: remotes.slice(0, 1),
    preview_sync_refs: { ...empty, deletions: [change('v1.0.0'), change('v2.0.0')] },
  });
  await emitBackendEvent(page, 'app-menu-action', 'sync-tags');
  const dialog = page.locator('#sync-tags-dialog');
  await expect(dialog.locator('#remote')).toHaveValue('origin');
  await expect(dialog.getByText('This remote has no tags.')).toBeVisible();
  await expect(dialog.getByRole('heading', { name: 'To delete (2)' })).toBeVisible();
  await expect(dialog.getByText('v1.0.0')).toBeVisible();
  await expect(page.locator('#sync-branches-dialog gn-modal[open]')).toHaveCount(0);
  await dialog.getByRole('button', { name: 'Cancel' }).click();
  await expect(dialog.locator('gn-modal[open]')).toHaveCount(0);
  expect(await findCommand(page, 'apply_sync_refs')).toHaveLength(0);
});

test('tag sync requires confirmation again after a changed comparison', async ({ page }) => {
  const first = { ...empty, remoteCount: 1, overwrites: [change('v1.0.0', { remoteOid })] };
  const second = { ...first, revision: 'second', deletions: [change('v2.0.0')] };
  await injectCommandMock(page, {
    get_remotes: remotes,
    preview_sync_refs: first,
    apply_sync_refs: { status: 'changed', preview: second },
  });
  await emitBackendEvent(page, 'app-menu-action', 'sync-tags');
  const dialog = page.locator('#sync-tags-dialog');
  const confirm = dialog.getByRole('button', { name: 'Confirm Sync' });
  await expect(confirm).toBeDisabled();
  await dialog.locator('#remote').selectOption('upstream');
  await expect(dialog.getByRole('heading', { name: 'To overwrite (1)' })).toBeVisible();
  await expect(dialog.getByText(localOid, { exact: true })).toBeVisible();
  await expect(dialog.getByText(remoteOid, { exact: true })).toBeVisible();
  await confirm.click();
  await expect(dialog.getByText('The comparison changed. Review it before confirming again.')).toBeVisible();
  await expect(dialog.getByRole('heading', { name: 'To delete (1)' })).toBeVisible();
  expect(await findCommand(page, 'apply_sync_refs')).toHaveLength(1);

  await injectCommandMock(page, { apply_sync_refs: { status: 'applied' } });
  await confirm.click();
  await expect(dialog.locator('gn-modal[open]')).toHaveCount(0);
  const calls = await findCommand(page, 'apply_sync_refs');
  expect(calls.map(({ args }) => args)).toMatchObject([
    { kind: 'tags', remote: 'upstream', revision: 'first' },
    { kind: 'tags', remote: 'upstream', revision: 'second' },
  ]);
  expect(await findCommand(page, 'push')).toHaveLength(0);
});

test('branch sync warns about local commits and requires a replacement for the current branch', async ({ page }) => {
  const branches: SyncRefsPreview = {
    ...empty,
    remoteCount: 1,
    newRefs: [change('release', { localOid: null, remoteOid })],
    deletions: [change('main', { outgoingCommits: 2 })],
    currentBranch: 'main',
    currentBranchDeleted: true,
    workingChanges: true,
    replacements: ['release'],
  };
  await injectCommandMock(page, {
    get_remotes: remotes.slice(0, 1),
    preview_sync_refs: branches,
    apply_sync_refs: { status: 'applied' },
  });
  await emitBackendEvent(page, 'app-menu-action', 'sync-branches');
  const dialog = page.locator('#sync-branches-dialog');
  const confirm = dialog.getByRole('button', { name: 'Confirm Sync' });
  await expect(dialog.getByText('Local commits leaving this branch (2)')).toBeVisible();
  await expect(dialog.getByText('Uncommitted changes in the current worktree may be discarded.')).toBeVisible();
  await expect(confirm).toBeDisabled();
  await dialog.locator('#replacement').selectOption('release');
  await confirm.click();
  await expect(dialog.locator('gn-modal[open]')).toHaveCount(0);
  expect((await findCommand(page, 'apply_sync_refs'))[0].args).toMatchObject({
    kind: 'branches', replacement: 'release',
  });
});

test('branch sync blocks branches checked out in another worktree', async ({ page }) => {
  await injectCommandMock(page, {
    get_remotes: remotes.slice(0, 1),
    preview_sync_refs: {
      ...empty,
      deletions: [change('feature', { worktree: '/other/worktree' })],
    },
  });
  await emitBackendEvent(page, 'app-menu-action', 'sync-branches');
  const dialog = page.locator('#sync-branches-dialog');
  await expect(dialog.getByText('Branch in use by another worktree at /other/worktree')).toBeVisible();
  await expect(dialog.getByRole('button', { name: 'Confirm Sync' })).toBeDisabled();
});
