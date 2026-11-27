/**
 * Settings Dialog External Tools Tests
 *
 * Two concerns share this file:
 *
 * - "None" must actually unset merge.tool / diff.tool in the repository
 *   config. Clearing only local component state leaves git (and the app's own
 *   launch-merge-tool / launch-diff-tool flows) on the old tool, and
 *   reopening Settings reloads it — silently reverting the user's choice.
 * - setMergeToolConfig / setDiffTool return a CommandResult and never throw,
 *   so a failed config write (read-only or locked .git/config) used to be
 *   completely silent: the control kept showing a value git never accepted
 *   and 'settings-changed' fired as if the write had landed. These tests pin
 *   the toast, the revert of both state and DOM control, the suppressed
 *   event, and that a stale write resolving late never clobbers a newer one.
 */

import { expect, fixture, html } from '@open-wc/testing';
import { repositoryStore } from '../../../stores/repository.store.ts';
import { uiStore } from '../../../stores/ui.store.ts';
import type { Repository } from '../../../types/git.types.ts';

type MockInvoke = (command: string, args?: unknown) => Promise<unknown>;

interface RecordedCall {
  command: string;
  args: Record<string, unknown> | undefined;
}

const calls: RecordedCall[] = [];
let responses: Record<string, unknown> = {};
const failures: Record<string, string> = {};
/** Whether each tool is configured outside the repository (global/system). */
const configuredGlobally = { mergeTool: false, diffTool: false };

/** Same purpose as `failures`, but for tests that only need a fixed message. */
let failingCommands: Set<string> = new Set();

/**
 * Commands parked until the test settles them, so two writes can be resolved
 * out of the order they were issued.
 */
let deferredCommands: Set<string> = new Set();
interface DeferredWrite {
  command: string;
  succeed: () => void;
  fail: () => void;
}
let deferredWrites: DeferredWrite[] = [];

const WRITE_ERROR = 'could not lock config file: Permission denied';

const mockInvoke: MockInvoke = async (command: string, args?: unknown) => {
  calls.push({ command, args: args as Record<string, unknown> | undefined });

  if (command === 'plugin:notification|is_permission_granted') return false;
  if (failingCommands.has(command)) {
    throw new Error(WRITE_ERROR);
  }
  if (failures[command] !== undefined) {
    throw new Error(failures[command]);
  }
  if (deferredCommands.has(command)) {
    return new Promise((resolve, reject) => {
      deferredWrites.push({
        command,
        succeed: () => resolve(null),
        fail: () => reject(new Error(WRITE_ERROR)),
      });
    });
  }

  // `git config --unset` writes the repository-local file only, while the
  // get_* commands read the effective value. A globally configured tool
  // therefore survives the unset, exactly as in the real backend.
  if (command === 'unset_git_config') {
    const key = (args as { key?: string } | undefined)?.key;
    if (key === 'merge.tool' && !configuredGlobally.mergeTool) {
      responses['get_merge_tool_config'] = { toolName: null, toolCmd: null };
    }
    if (key === 'diff.tool' && !configuredGlobally.diffTool) {
      responses['get_diff_tool'] = { tool: null, cmd: null, prompt: false };
    }
  }

  if (command in responses) return responses[command];

  switch (command) {
    case 'get_ai_providers':
      return [];
    case 'get_app_version':
      return '0.1.0';
    case 'get_settings':
      return {};
    case 'get_mcp_status':
      return { servers: [], totalTools: 0 };
    case 'get_graph_color_schemes':
      return [];
    case 'get_merge_tool_config':
      return { toolName: null, toolCmd: null };
    case 'get_diff_tool':
      return { tool: null, cmd: null, prompt: false };
    case 'get_available_merge_tools':
      return [
        { name: 'meld', displayName: 'Meld', command: 'meld', available: true },
        { name: 'vimdiff', displayName: 'Vimdiff', command: 'vimdiff', available: true },
      ];
    case 'list_diff_tools':
      return [
        { name: 'meld', command: 'meld', available: true },
        { name: 'vimdiff', command: 'vimdiff', available: true },
      ];
    case 'set_merge_tool_config':
    case 'set_diff_tool':
    case 'unset_git_config':
      return null;
    default:
      return null;
  }
};

let callbackId = 0;
(globalThis as Record<string, unknown>).__TAURI_INTERNALS__ = {
  invoke: mockInvoke,
  transformCallback: () => callbackId++,
  convertFileSrc: (path: string) => path,
};

// Import AFTER setting up the mock
import '../gn-modal.ts';
import '../gn-settings-dialog.ts';
import type { GnModal } from '../gn-modal.ts';
import type { GnSettingsDialog } from '../gn-settings-dialog.ts';

async function waitUntil(predicate: () => boolean, what: string): Promise<void> {
  for (let i = 0; i < 200; i++) {
    if (predicate()) return;
    await new Promise((r) => setTimeout(r, 0));
  }
  throw new Error(`timed out waiting for ${what}`);
}

/** Give any pending handler continuation plenty of turns to run. */
async function settle(): Promise<void> {
  for (let i = 0; i < 50; i++) {
    await new Promise((r) => setTimeout(r, 0));
  }
}

interface ToolState {
  mergeToolName: string | null;
  mergeToolCmd: string | null;
  diffToolName: string | null;
  diffToolCmd: string | null;
}

function toolState(el: GnSettingsDialog): ToolState {
  return el as unknown as ToolState;
}

function settingRow(el: GnSettingsDialog, name: string): Element {
  const rows = Array.from(el.shadowRoot?.querySelectorAll('.setting-row') ?? []);
  const row = rows.find(
    (r) => r.querySelector('.setting-name')?.textContent?.trim() === name,
  );
  if (!row) throw new Error(`no setting row named "${name}"`);
  return row;
}

function rowSelect(el: GnSettingsDialog, name: string): HTMLSelectElement {
  const select = settingRow(el, name).querySelector('select');
  if (!select) throw new Error(`no select in setting row "${name}"`);
  return select;
}

function rowInput(el: GnSettingsDialog, name: string): HTMLInputElement {
  const input = settingRow(el, name).querySelector('input');
  if (!input) throw new Error(`no input in setting row "${name}"`);
  return input;
}

function change(control: HTMLElement): void {
  control.dispatchEvent(new Event('change', { bubbles: true }));
}

function errorToasts(): string[] {
  return uiStore
    .getState()
    .toasts.filter((t) => t.type === 'error')
    .map((t) => t.message);
}

function mockRepo(path: string, name: string): Repository {
  return {
    path,
    name,
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

function unsetCalls(): RecordedCall[] {
  return calls.filter((c) => c.command === 'unset_git_config');
}

/** Find the <select> in the setting row whose name matches `label`. */
function toolSelect(el: GnSettingsDialog, label: string): HTMLSelectElement {
  const rows = Array.from(el.shadowRoot?.querySelectorAll('.setting-row') ?? []);
  const row = rows.find(
    (r) => r.querySelector('.setting-name')?.textContent?.trim() === label,
  );
  const select = row?.querySelector('select') as HTMLSelectElement | null | undefined;
  expect(select, `${label} select exists`).to.exist;
  return select as HTMLSelectElement;
}

async function renderDialog(): Promise<GnSettingsDialog> {
  const el = await fixture<GnSettingsDialog>(
    html`<gn-settings-dialog></gn-settings-dialog>`,
  );
  await el.updateComplete;
  // Wait for the async external-tools load kicked off by connectedCallback.
  for (let i = 0; i < 50; i++) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    if ((el as any).loadingTools === false && (el as any).loadingEditor === false) break;
    await new Promise((r) => setTimeout(r, 5));
    await el.updateComplete;
  }
  await el.updateComplete;
  return el;
}

/** Drive the real rendered <select> the way a user would. */
async function chooseNone(el: GnSettingsDialog, label: string): Promise<HTMLSelectElement> {
  const select = toolSelect(el, label);
  select.value = '';
  const handler = label === 'Merge Tool' ? 'handleMergeToolChange' : 'handleDiffToolChange';
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  await (el as any)[handler]({ target: select } as unknown as Event);
  await el.updateComplete;
  return select;
}

function lastToast(): { type: string; message: string } | undefined {
  const toasts = uiStore.getState().toasts;
  return toasts[toasts.length - 1];
}

describe('gn-settings-dialog external tools write failures', () => {
  const REPO_PATH = '/test/external-tools-repo';
  let el: GnSettingsDialog;
  let settingsChangedCount: number;
  let onSettingsChanged: () => void;

  beforeEach(async () => {
    calls.length = 0;
    failingCommands = new Set();
    for (const key of Object.keys(failures)) delete failures[key];
    deferredCommands = new Set();
    deferredWrites = [];
    responses = {
      get_merge_tool_config: { toolName: null, toolCmd: null },
      get_diff_tool: { tool: null, cmd: null, prompt: false },
      get_available_merge_tools: [
        { name: 'meld', displayName: 'Meld', command: 'meld', available: true },
        { name: 'vimdiff', displayName: 'Vimdiff', command: 'vimdiff', available: true },
      ],
      list_diff_tools: [
        { name: 'meld', command: 'meld', available: true },
        { name: 'vimdiff', command: 'vimdiff', available: true },
      ],
      set_merge_tool_config: null,
      set_diff_tool: null,
      unset_git_config: null,
    };
    uiStore.setState({ toasts: [] });
    repositoryStore.getState().reset();
    repositoryStore.getState().addRepository({
      path: REPO_PATH,
      name: 'external-tools-repo',
      isValid: true,
      isBare: false,
      headRef: 'main',
      detachedHeadOid: null,
      state: 'clean',
      isShallow: false,
      isPartialClone: false,
      cloneFilter: null,
    });

    settingsChangedCount = 0;
    onSettingsChanged = () => {
      settingsChangedCount += 1;
    };
    window.addEventListener('settings-changed', onSettingsChanged);

    el = await fixture<GnSettingsDialog>(html`<gn-settings-dialog></gn-settings-dialog>`);
    await el.updateComplete;
    await waitUntil(
      () => !!el.shadowRoot?.querySelector('option[value="vimdiff"]'),
      'the external tools lists to load',
    );
  });

  afterEach(() => {
    window.removeEventListener('settings-changed', onSettingsChanged);
    repositoryStore.getState().reset();
    uiStore.setState({ toasts: [] });
  });

  it('shows an error toast when saving the merge tool fails', async () => {
    failingCommands.add('set_merge_tool_config');

    const select = rowSelect(el, 'Merge Tool');
    select.value = 'meld';
    change(select);

    await waitUntil(() => errorToasts().length > 0, 'the merge tool error toast');
    expect(errorToasts()[0]).to.contain('Failed to save merge tool');
    expect(errorToasts()[0]).to.contain('Permission denied');
  });

  it('puts the merge tool select back on the saved value when the write fails', async () => {
    failingCommands.add('set_merge_tool_config');

    const select = rowSelect(el, 'Merge Tool');
    select.value = 'meld';
    change(select);

    await waitUntil(() => errorToasts().length > 0, 'the merge tool error toast');
    await el.updateComplete;

    expect(select.value, 'select reverted to None').to.equal('');
    expect(toolState(el).mergeToolName, 'mergeToolName reverted').to.be.null;
    expect(settingsChangedCount, 'settings-changed suppressed on failure').to.equal(0);
  });

  it('restores the previously saved merge tool, not None, when a later write fails', async () => {
    const select = rowSelect(el, 'Merge Tool');
    select.value = 'meld';
    change(select);

    await waitUntil(() => settingsChangedCount === 1, 'the first merge tool save');
    await el.updateComplete;
    expect(select.value).to.equal('meld');

    failingCommands.add('set_merge_tool_config');
    select.value = 'vimdiff';
    change(select);

    await waitUntil(() => errorToasts().length > 0, 'the merge tool error toast');
    await el.updateComplete;

    expect(select.value, 'reverted to the last saved tool').to.equal('meld');
    expect(toolState(el).mergeToolName).to.equal('meld');
    expect(settingsChangedCount, 'no second settings-changed').to.equal(1);
  });

  it('shows an error toast and restores the saved merge tool command when the custom command write fails', async () => {
    const select = rowSelect(el, 'Merge Tool');
    select.value = '__custom__';
    change(select);
    await el.updateComplete;

    failingCommands.add('set_merge_tool_config');
    const input = rowInput(el, 'Merge Tool Command');
    input.value = '/usr/bin/meld $LOCAL $REMOTE $MERGED';
    change(input);

    await waitUntil(() => errorToasts().length > 0, 'the merge tool command error toast');
    await el.updateComplete;

    expect(errorToasts()[0]).to.contain('Failed to save merge tool command');
    expect(errorToasts()[0]).to.contain('Permission denied');
    expect(input.value, 'command input reverted').to.equal('');
    expect(toolState(el).mergeToolCmd, 'mergeToolCmd reverted').to.equal('');
    expect(toolState(el).mergeToolName, 'custom row stays open for a retry').to.equal('__custom__');
    expect(settingsChangedCount, 'settings-changed suppressed on failure').to.equal(0);
  });

  it('shows an error toast and reverts the diff tool select when the write fails', async () => {
    failingCommands.add('set_diff_tool');

    const select = rowSelect(el, 'Diff Tool');
    select.value = 'meld';
    change(select);

    await waitUntil(() => errorToasts().length > 0, 'the diff tool error toast');
    await el.updateComplete;

    expect(errorToasts()[0]).to.contain('Failed to save diff tool');
    expect(errorToasts()[0]).to.contain('Permission denied');
    expect(select.value, 'select reverted to None').to.equal('');
    expect(toolState(el).diffToolName, 'diffToolName reverted').to.be.null;
    expect(settingsChangedCount, 'settings-changed suppressed on failure').to.equal(0);
  });

  it('shows an error toast and restores the saved diff tool command when the custom command write fails', async () => {
    const select = rowSelect(el, 'Diff Tool');
    select.value = '__custom__';
    change(select);
    await el.updateComplete;

    failingCommands.add('set_diff_tool');
    const input = rowInput(el, 'Diff Tool Command');
    input.value = '/usr/bin/meld $LOCAL $REMOTE';
    change(input);

    await waitUntil(() => errorToasts().length > 0, 'the diff tool command error toast');
    await el.updateComplete;

    expect(errorToasts()[0]).to.contain('Failed to save diff tool command');
    expect(input.value, 'command input reverted').to.equal('');
    expect(toolState(el).diffToolCmd, 'diffToolCmd reverted').to.equal('');
    expect(toolState(el).diffToolName, 'custom row stays open for a retry').to.equal('__custom__');
    expect(settingsChangedCount, 'settings-changed suppressed on failure').to.equal(0);
  });

  it('keeps the newer saved merge tool when an older failed write resolves late', async () => {
    deferredCommands.add('set_merge_tool_config');

    const select = rowSelect(el, 'Merge Tool');
    select.value = 'meld';
    change(select);
    await waitUntil(() => deferredWrites.length === 1, 'the first merge tool write');

    select.value = 'vimdiff';
    change(select);
    await waitUntil(() => deferredWrites.length === 2, 'the second merge tool write');

    // The newer write lands first and succeeds...
    deferredWrites[1].succeed();
    await waitUntil(() => settingsChangedCount === 1, 'the newer merge tool save');
    await el.updateComplete;
    expect(select.value).to.equal('vimdiff');

    // ...then the superseded write fails. It must not roll the control back.
    deferredWrites[0].fail();
    await settle();
    await el.updateComplete;

    expect(select.value, 'newer saved tool survives').to.equal('vimdiff');
    expect(toolState(el).mergeToolName).to.equal('vimdiff');
    expect(errorToasts(), 'no rollback toast for a superseded write').to.be.empty;
    expect(settingsChangedCount, 'only the newer save dispatched').to.equal(1);
  });

  it('keeps the newer saved diff tool when an older failed write resolves late', async () => {
    deferredCommands.add('set_diff_tool');

    const select = rowSelect(el, 'Diff Tool');
    select.value = 'meld';
    change(select);
    await waitUntil(() => deferredWrites.length === 1, 'the first diff tool write');

    select.value = 'vimdiff';
    change(select);
    await waitUntil(() => deferredWrites.length === 2, 'the second diff tool write');

    deferredWrites[1].succeed();
    await waitUntil(() => settingsChangedCount === 1, 'the newer diff tool save');
    await el.updateComplete;
    expect(select.value).to.equal('vimdiff');

    deferredWrites[0].fail();
    await settle();
    await el.updateComplete;

    expect(select.value, 'newer saved tool survives').to.equal('vimdiff');
    expect(toolState(el).diffToolName).to.equal('vimdiff');
    expect(errorToasts(), 'no rollback toast for a superseded write').to.be.empty;
    expect(settingsChangedCount, 'only the newer save dispatched').to.equal(1);
  });

  it('leaves the merge tool on None when an in-flight write fails after the user picks None', async () => {
    const select = rowSelect(el, 'Merge Tool');
    select.value = 'meld';
    change(select);
    await waitUntil(() => settingsChangedCount === 1, 'the first merge tool save');
    await el.updateComplete;

    deferredCommands.add('set_merge_tool_config');
    select.value = 'vimdiff';
    change(select);
    await waitUntil(() => deferredWrites.length === 1, 'the merge tool write');

    // The user gives up on external merge tools before the write comes back.
    select.value = '';
    change(select);
    await waitUntil(() => settingsChangedCount === 2, 'the None clear to land');
    await el.updateComplete;

    deferredWrites[0].fail();
    await settle();
    await el.updateComplete;

    expect(select.value, 'the None choice survives').to.equal('');
    expect(toolState(el).mergeToolName).to.be.null;
    expect(errorToasts(), 'no rollback toast for a superseded write').to.be.empty;
    // One dispatch for the initial 'meld' save, one for the real unset behind
    // "None" (it round-trips through the backend and dispatches on success
    // just like any other write) — the superseded 'vimdiff' write must not add
    // a third.
    expect(settingsChangedCount, 'only the meld save and the None clear dispatched').to.equal(2);
  });

  it('does not roll the custom diff tool command back over a newer tool selection', async () => {
    const select = rowSelect(el, 'Diff Tool');
    select.value = '__custom__';
    change(select);
    await el.updateComplete;

    deferredCommands.add('set_diff_tool');
    const input = rowInput(el, 'Diff Tool Command');
    input.value = '/usr/bin/meld $LOCAL $REMOTE';
    change(input);
    await waitUntil(() => deferredWrites.length === 1, 'the custom command write');

    // The user switches to a named tool while the command write is still open.
    select.value = 'vimdiff';
    change(select);
    await waitUntil(() => deferredWrites.length === 2, 'the named tool write');
    deferredWrites[1].succeed();
    await waitUntil(() => settingsChangedCount === 1, 'the named tool save');
    await el.updateComplete;

    deferredWrites[0].fail();
    await settle();
    await el.updateComplete;

    expect(select.value, 'the named tool survives').to.equal('vimdiff');
    expect(toolState(el).diffToolName).to.equal('vimdiff');
    expect(errorToasts(), 'no rollback toast for a superseded write').to.be.empty;
    expect(settingsChangedCount).to.equal(1);
  });

  it('dispatches settings-changed and keeps the value when the merge tool write succeeds', async () => {
    const select = rowSelect(el, 'Merge Tool');
    select.value = 'meld';
    change(select);

    await waitUntil(() => settingsChangedCount === 1, 'the merge tool save');
    await el.updateComplete;

    expect(select.value).to.equal('meld');
    expect(toolState(el).mergeToolName).to.equal('meld');
    expect(errorToasts(), 'no error toast on success').to.be.empty;
  });
});

describe('gn-settings-dialog external tools "None"', () => {
  const NONE_REPO_PATH = '/repo';

  beforeEach(() => {
    calls.length = 0;
    failingCommands = new Set();
    deferredCommands = new Set();
    deferredWrites = [];
    for (const key of Object.keys(failures)) delete failures[key];
    configuredGlobally.mergeTool = false;
    configuredGlobally.diffTool = false;
    responses = {
      get_merge_tool_config: { toolName: 'meld', toolCmd: null },
      get_available_merge_tools: [{ name: 'meld', displayName: 'Meld', available: true }],
      get_diff_tool: { tool: 'vscode', cmd: null, prompt: false },
      list_diff_tools: [{ name: 'vscode', command: 'code', available: true }],
      unset_git_config: null,
      set_merge_tool_config: null,
      set_diff_tool: null,
    };
    uiStore.setState({ toasts: [] });
    repositoryStore.getState().reset();
    repositoryStore.getState().addRepository(mockRepo(NONE_REPO_PATH, 'repo'));
  });

  afterEach(() => {
    repositoryStore.getState().reset();
    uiStore.setState({ toasts: [] });
  });

  it('shows the configured tools as selected when the dialog opens', async () => {
    const el = await renderDialog();

    // The <select>'s .value binding commits before the <option>s rendered in the
    // same update, so without an explicit re-sync a configured tool shows "None".
    expect(toolSelect(el, 'Merge Tool').value, 'merge select').to.equal('meld');
    expect(toolSelect(el, 'Diff Tool').value, 'diff select').to.equal('vscode');
  });

  it('keeps the merge tool selected when merge.tool is configured outside the repository', async () => {
    configuredGlobally.mergeTool = true;
    const el = await renderDialog();

    let eventFired = false;
    const listener = () => { eventFired = true; };
    window.addEventListener('settings-changed', listener);
    const select = await chooseNone(el, 'Merge Tool');
    window.removeEventListener('settings-changed', listener);

    // The local unset is still attempted, but merge.tool survives it, so the UI
    // must not claim the tool is gone while git would still launch it.
    expect(unsetCalls(), 'local unset attempted').to.have.lengthOf(1);
    expect(eventFired, 'settings-changed NOT dispatched').to.be.false;
    expect(lastToast()?.type, 'error toast shown').to.equal('error');
    expect(lastToast()?.message).to.contain('global or system');
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    expect((el as any).mergeToolName, 'still configured').to.equal('meld');
    expect(select.value, 'select still shows the surviving tool').to.equal('meld');
  });

  it('keeps the diff tool selected when diff.tool is configured outside the repository', async () => {
    configuredGlobally.diffTool = true;
    const el = await renderDialog();

    let eventFired = false;
    const listener = () => { eventFired = true; };
    window.addEventListener('settings-changed', listener);
    const select = await chooseNone(el, 'Diff Tool');
    window.removeEventListener('settings-changed', listener);

    expect(unsetCalls(), 'local unset attempted').to.have.lengthOf(1);
    expect(eventFired, 'settings-changed NOT dispatched').to.be.false;
    expect(lastToast()?.type, 'error toast shown').to.equal('error');
    expect(lastToast()?.message).to.contain('global or system');
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    expect((el as any).diffToolName, 'still configured').to.equal('vscode');
    expect(select.value, 'select still shows the surviving tool').to.equal('vscode');
  });

  it('unsets merge.tool when Merge Tool is set to None', async () => {
    const el = await renderDialog();
    await chooseNone(el, 'Merge Tool');

    const unsets = unsetCalls();
    expect(unsets, 'unset_git_config was invoked').to.have.lengthOf(1);
    expect(unsets[0].args).to.deep.equal({ path: NONE_REPO_PATH, key: 'merge.tool', global: undefined });
  });

  it('dispatches settings-changed and clears the merge tool after a successful unset', async () => {
    const el = await renderDialog();
    let eventFired = false;
    const listener = () => { eventFired = true; };
    window.addEventListener('settings-changed', listener);

    await chooseNone(el, 'Merge Tool');
    window.removeEventListener('settings-changed', listener);

    expect(eventFired, 'settings-changed dispatched').to.be.true;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    expect((el as any).mergeToolName).to.be.null;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    expect((el as any).mergeToolCmd).to.be.null;
  });

  it('shows an error toast and keeps the configured merge tool when the unset fails', async () => {
    const el = await renderDialog();
    failures['unset_git_config'] = 'error: could not unset merge.tool';

    let eventFired = false;
    const listener = () => { eventFired = true; };
    window.addEventListener('settings-changed', listener);
    const select = await chooseNone(el, 'Merge Tool');
    window.removeEventListener('settings-changed', listener);

    expect(eventFired, 'settings-changed NOT dispatched on failure').to.be.false;
    expect(lastToast()?.type, 'error toast shown').to.equal('error');
    expect(lastToast()?.message).to.contain('could not unset');
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    expect((el as any).mergeToolName, 'still configured').to.equal('meld');
    expect(select.value, 'select reverted to the configured tool').to.equal('meld');
  });

  it('unsets diff.tool when Diff Tool is set to None', async () => {
    const el = await renderDialog();
    await chooseNone(el, 'Diff Tool');

    const unsets = unsetCalls();
    expect(unsets, 'unset_git_config was invoked').to.have.lengthOf(1);
    expect(unsets[0].args).to.deep.equal({ path: NONE_REPO_PATH, key: 'diff.tool', global: undefined });
  });

  it('dispatches settings-changed and clears the diff tool after a successful unset', async () => {
    const el = await renderDialog();
    let eventFired = false;
    const listener = () => { eventFired = true; };
    window.addEventListener('settings-changed', listener);

    await chooseNone(el, 'Diff Tool');
    window.removeEventListener('settings-changed', listener);

    expect(eventFired, 'settings-changed dispatched').to.be.true;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    expect((el as any).diffToolName).to.be.null;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    expect((el as any).diffToolCmd).to.be.null;
  });

  it('shows an error toast and keeps the configured diff tool when the unset fails', async () => {
    const el = await renderDialog();
    failures['unset_git_config'] = 'error: could not unset diff.tool';

    let eventFired = false;
    const listener = () => { eventFired = true; };
    window.addEventListener('settings-changed', listener);
    const select = await chooseNone(el, 'Diff Tool');
    window.removeEventListener('settings-changed', listener);

    expect(eventFired, 'settings-changed NOT dispatched on failure').to.be.false;
    expect(lastToast()?.type, 'error toast shown').to.equal('error');
    expect(lastToast()?.message).to.contain('could not unset');
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    expect((el as any).diffToolName, 'still configured').to.equal('vscode');
    expect(select.value, 'select reverted to the configured tool').to.equal('vscode');
  });

  it('unsets merge.tool when None is chosen after a Custom tool', async () => {
    const el = await renderDialog();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (el as any).mergeToolName = '__custom__';
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (el as any).mergeToolCmd = 'meld $LOCAL $REMOTE $MERGED';
    await el.updateComplete;

    await chooseNone(el, 'Merge Tool');

    const unsets = unsetCalls();
    expect(unsets, 'unset_git_config was invoked').to.have.lengthOf(1);
    expect(unsets[0].args).to.include({ key: 'merge.tool' });
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    expect((el as any).mergeToolCmd).to.be.null;
  });

  it('still issues the unset when no tool was configured', async () => {
    responses['get_merge_tool_config'] = { toolName: null, toolCmd: null };
    const el = await renderDialog();

    let eventFired = false;
    const listener = () => { eventFired = true; };
    window.addEventListener('settings-changed', listener);
    await chooseNone(el, 'Merge Tool');
    window.removeEventListener('settings-changed', listener);

    // The backend treats a missing key as a benign no-op, so the write is safe
    // and the UI stays honest about what is configured.
    const unsets = unsetCalls();
    expect(unsets).to.have.lengthOf(1);
    expect(unsets[0].args).to.include({ key: 'merge.tool' });
    expect(eventFired, 'settings-changed dispatched').to.be.true;
  });

  it('does nothing when no repository is open', async () => {
    const el = await renderDialog();
    repositoryStore.getState().reset();
    await el.updateComplete;

    const select = document.createElement('select');
    select.innerHTML = '<option value=""></option>';
    select.value = '';
    calls.length = 0;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    await (el as any).handleMergeToolChange({ target: select } as unknown as Event);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    await (el as any).handleDiffToolChange({ target: select } as unknown as Event);

    expect(unsetCalls(), 'no config write without an active repository').to.have.lengthOf(0);
    expect(uiStore.getState().toasts, 'no toast').to.have.lengthOf(0);
  });
});

describe('gn-settings-dialog external editor', () => {
  const EDITOR_REPO_PATH = '/test/editor-repo';
  let settingsChangedCount = 0;
  let onSettingsChanged: () => void;

  beforeEach(() => {
    calls.length = 0;
    failingCommands = new Set();
    deferredCommands = new Set();
    deferredWrites = [];
    for (const key of Object.keys(failures)) delete failures[key];
    configuredGlobally.mergeTool = false;
    configuredGlobally.diffTool = false;
    responses = {
      get_available_editors: [
        { id: 'vscode', displayName: 'VS Code', command: 'code', available: true },
        { id: 'cursor', displayName: 'Cursor', command: 'cursor', available: false },
        { id: 'zed', displayName: 'Zed', command: 'zed', available: true },
      ],
      get_editor_config: { editor: null, visual: null },
      set_editor_config: { success: true, message: 'Editor updated' },
      get_merge_tool_config: { toolName: null, toolCmd: null },
      get_diff_tool: { tool: null, cmd: null, prompt: false },
      get_available_merge_tools: [],
      list_diff_tools: [],
    };
    uiStore.setState({ toasts: [] });
    repositoryStore.getState().reset();
    settingsChangedCount = 0;
    onSettingsChanged = () => {
      settingsChangedCount += 1;
    };
    window.addEventListener('settings-changed', onSettingsChanged);
  });

  afterEach(() => {
    window.removeEventListener('settings-changed', onSettingsChanged);
    repositoryStore.getState().reset();
    uiStore.setState({ toasts: [] });
  });

  it('renders the Editor selector even when no repository is open', async () => {
    const el = await renderDialog();

    const editorSelect = el.shadowRoot?.querySelector<HTMLSelectElement>('#external-editor-select');
    expect(editorSelect).to.exist;
    expect(editorSelect?.disabled).to.equal(false);

    const getEditorCalls = calls.filter((c) => c.command === 'get_editor_config');
    expect(getEditorCalls).to.have.lengthOf(1);
    expect(getEditorCalls[0].args).to.deep.equal({ path: '', global: true });

    const section = el.shadowRoot?.querySelector('#external-tools-section');
    expect(section?.textContent).to.contain('Open a repository to configure merge and diff tools');
    expect(el.shadowRoot?.querySelector('#merge-tool-select')).to.equal(null);
  });

  it('tolerates null responses from legacy mocks for editor commands', async () => {
    delete responses['get_available_editors'];
    delete responses['get_editor_config'];

    const el = await renderDialog();
    const editorSelect = el.shadowRoot?.querySelector<HTMLSelectElement>('#external-editor-select');
    expect(editorSelect).to.exist;
    expect(editorSelect?.value).to.equal('');
    const optionValues = Array.from(editorSelect?.options ?? []).map((o) => o.value);
    expect(optionValues).to.deep.equal(['', '__custom__']);
  });

  it('shows only detected available editors (without None) and falls back to all OS presets when none are detected', async () => {
    const el = await renderDialog();
    const editorSelect = el.shadowRoot!.querySelector<HTMLSelectElement>('#external-editor-select')!;
    const options = Array.from(editorSelect.options);
    const values = options.map((o) => o.value);
    const labels = options.map((o) => o.textContent?.trim());

    expect(values).to.deep.equal(['', 'code', 'zed', '__custom__']);
    expect(options[0].disabled).to.equal(true);
    expect(labels[0]).to.equal('Select an editor...');
    expect(labels).to.not.include('None');
    expect(labels).to.not.include('Cursor (cursor)');

    responses['get_available_editors'] = [
      { id: 'vscode', displayName: 'VS Code', command: 'code', available: false },
      { id: 'cursor', displayName: 'Cursor', command: 'cursor', available: false },
    ];
    const fallbackEl = await renderDialog();
    const fallbackSelect = fallbackEl.shadowRoot!.querySelector<HTMLSelectElement>('#external-editor-select')!;
    expect(Array.from(fallbackSelect.options).map((o) => o.value)).to.deep.equal([
      '',
      'code',
      'cursor',
      '__custom__',
    ]);
  });

  it('saves a selected editor globally and dispatches settings-changed', async () => {
    repositoryStore.getState().addRepository(mockRepo(EDITOR_REPO_PATH, 'editor-repo'));
    const el = await renderDialog();

    const editorSelect = el.shadowRoot!.querySelector<HTMLSelectElement>('#external-editor-select')!;
    editorSelect.value = 'zed';
    change(editorSelect);

    await waitUntil(() => settingsChangedCount === 1, 'settings-changed after editor save');
    await el.updateComplete;

    const setCalls = calls.filter((c) => c.command === 'set_editor_config');
    expect(setCalls).to.have.lengthOf(1);
    expect(setCalls[0].args).to.deep.equal({
      path: EDITOR_REPO_PATH,
      editor: 'zed',
      global: true,
    });
    expect(editorSelect.value).to.equal('zed');
    expect(editorSelect.querySelector('option[value=""]')).to.equal(null);
  });

  it('preserves a saved custom editor command across reloads and restores empty submissions', async () => {
    responses['get_editor_config'] = { editor: 'nvim --wait', visual: null };
    const el = await renderDialog();

    const editorSelect = el.shadowRoot!.querySelector<HTMLSelectElement>('#external-editor-select')!;
    expect(editorSelect.value).to.equal('__custom__');

    const customInput = el.shadowRoot!.querySelector<HTMLInputElement>('#external-editor-custom-input')!;
    expect(customInput).to.exist;
    expect(customInput.value).to.equal('nvim --wait');

    customInput.value = '   ';
    change(customInput);
    await settle();

    expect(customInput.value).to.equal('nvim --wait');
    expect(calls.filter((c) => c.command === 'set_editor_config')).to.have.lengthOf(0);

    customInput.value = 'emacs -nw';
    change(customInput);
    await waitUntil(() => settingsChangedCount === 1, 'settings-changed after custom command save');

    const setCalls = calls.filter((c) => c.command === 'set_editor_config');
    expect(setCalls).to.have.lengthOf(1);
    expect(setCalls[0].args).to.deep.equal({ path: '', editor: 'emacs -nw', global: true });
  });

  it('focuses the custom command input when Custom... is chosen without overwriting core.editor', async () => {
    responses['get_editor_config'] = { editor: 'code', visual: null };
    const el = await renderDialog();

    const editorSelect = el.shadowRoot!.querySelector<HTMLSelectElement>('#external-editor-select')!;
    editorSelect.value = '__custom__';
    change(editorSelect);

    await waitUntil(
      () => !!el.shadowRoot?.querySelector('#external-editor-custom-input'),
      'custom input to appear',
    );
    await el.updateComplete;

    const customInput = el.shadowRoot!.querySelector<HTMLInputElement>('#external-editor-custom-input')!;
    expect(customInput.value).to.equal('');
    expect(el.shadowRoot?.activeElement).to.equal(customInput);
    expect(calls.filter((c) => c.command === 'set_editor_config')).to.have.lengthOf(0);
  });

  it('rolls back editor selection and shows an error toast when saving fails', async () => {
    responses['get_editor_config'] = { editor: 'code', visual: null };
    responses['set_editor_config'] = { success: false, message: 'git config failed' };
    const el = await renderDialog();

    const editorSelect = el.shadowRoot!.querySelector<HTMLSelectElement>('#external-editor-select')!;
    editorSelect.value = 'zed';
    change(editorSelect);

    await waitUntil(() => errorToasts().length > 0, 'editor save error toast');
    await el.updateComplete;

    expect(errorToasts()[0]).to.contain('Failed to save editor');
    expect(errorToasts()[0]).to.contain('git config failed');
    expect(editorSelect.value).to.equal('code');
    expect(settingsChangedCount).to.equal(0);
  });

  it('scrolls to external tools and keeps focus on #external-editor-select inside gn-modal when focusSection is external-tools', async () => {
    const modal = await fixture<GnModal>(html`
      <gn-modal open modalTitle="Settings">
        <gn-settings-dialog .focusSection=${'external-tools'}></gn-settings-dialog>
      </gn-modal>
    `);
    const dialog = modal.querySelector<GnSettingsDialog>('gn-settings-dialog')!;
    const section = dialog.shadowRoot?.querySelector<HTMLElement>('#external-tools-section');
    let scrolled = false;
    if (section) {
      section.scrollIntoView = () => {
        scrolled = true;
      };
    }

    await waitUntil(
      () => dialog.shadowRoot?.querySelector<HTMLSelectElement>('#external-editor-select')?.disabled === false,
      'editor config to load',
    );
    await dialog.updateComplete;
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

    const editorSelect = dialog.shadowRoot!.querySelector<HTMLSelectElement>('#external-editor-select')!;
    expect(scrolled).to.equal(true);
    expect(dialog.shadowRoot?.activeElement).to.equal(editorSelect);
  });
});
