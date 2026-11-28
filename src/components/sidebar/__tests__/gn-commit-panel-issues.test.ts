import { expect, fixture, html, waitUntil } from '@open-wc/testing';
import { settingsStore } from '../../../stores/settings.store.ts';
import { invalidateIssueTrackerCache } from '../../../services/issue-tracker.service.ts';
import { GnCommitPanel } from '../gn-commit-panel.ts';
import '../gn-commit-panel.ts';

describe('commit issue suggestions', () => {
  let el: GnCommitPanel;
  let mode: 'ready' | 'no-provider' | 'unauthenticated' | 'error' | 'empty';
  let committed: string;
  let finishList: (() => void) | undefined;
  let pauseList = false;
  const globals = globalThis as Record<string, unknown>;
  const previous = globals.__TAURI_INTERNALS__;
  const find = <T extends HTMLElement = HTMLButtonElement>(selector: string) => el.shadowRoot!.querySelector<T>(selector)!;
  const key = (input: HTMLElement, value: string) => input.dispatchEvent(new KeyboardEvent('keydown', { key: value, bubbles: true, cancelable: true }));
  const type = (input: HTMLTextAreaElement | HTMLInputElement, value: string) => {
    input.value = value;
    input.dispatchEvent(new Event('input', { bubbles: true }));
  };
  const open = () => find('.issue-btn').click();
  const ready = () => waitUntil(() => Boolean(find('.issue-item')));
  beforeEach(async () => {
    invalidateIssueTrackerCache();
    mode = 'ready';
    committed = '';
    pauseList = false;
    finishList = undefined;
    globals.__TAURI_INTERNALS__ = { transformCallback: () => 0, invoke: async (cmd: string, args: Record<string, unknown>) => {
      switch (cmd) {
        case 'detect_github_repo': return mode === 'no-provider' ? null : { owner: 'org', repo: args.path };
        case 'get_keyring_token': return mode === 'unauthenticated' ? null : 'token';
        case 'list_issues':
          if (pauseList) await new Promise<void>(resolve => { finishList = resolve; });
          if (mode === 'error') throw new Error('Temporary failure');
          return { issues: mode === 'empty' ? [] : ['Editor', 'Sidebar'].map((title, i) => ({
            title, number: 23 + i, state: 'open', htmlUrl: '', labels: [],
          })) };
        case 'get_user_identity': return { name: 'Ada', email: 'ada@example.org' };
        case 'list_templates': case 'get_conventional_types': return [];
        case 'create_commit': committed = String(args.message); return { shortId: 'abc' };
        default: return null;
      }
    } };
    el = await fixture<GnCommitPanel>(html`<gn-commit-panel .repositoryPath=${'/a'} .stagedCount=${1}></gn-commit-panel>`);
    await waitUntil(() => !find<HTMLInputElement>('.signoff-toggle input').disabled);
  });
  afterEach(() => {
    globals.__TAURI_INTERNALS__ = previous;
    settingsStore.getState().setOfflineMode(false);
    settingsStore.getState().setRemoteAllowlist([]);
  });

  it('uses the Co-authors button style and opens a filtered, keyboard-driven popover', async () => {
    const button = find('.options-row .issue-btn');
    const neighbor = find('.coauthor-wrapper button');
    for (const prop of ['font-size', 'padding', 'border-radius', 'height']) {
      expect(getComputedStyle(button).getPropertyValue(prop)).to.equal(getComputedStyle(neighbor).getPropertyValue(prop));
    }
    open();
    await ready();
    const filter = find<HTMLInputElement>('.issue-filter');
    expect(el.shadowRoot!.activeElement).to.equal(filter);
    type(filter, 'SIDE');
    await el.updateComplete;
    expect(find('.issue-item').textContent).to.contain('#24');
    type(filter, '');
    key(filter, 'ArrowDown');
    key(filter, 'ArrowUp');
    key(filter, 'ArrowDown');
    key(filter, 'Enter');
    await el.updateComplete;
    expect(find<HTMLTextAreaElement>('.description-input').value).to.equal('Refs #24');
  });

  it('replaces inline triggers in both fields and keeps author trailers in the commit', async () => {
    const summary = find<HTMLTextAreaElement>('.summary-input');
    const body = find<HTMLTextAreaElement>('.description-input');
    type(summary, 'Fix editor #23');
    await ready();
    expect(find('.issue-filter')).to.equal(null);
    key(summary, 'Enter');
    await el.updateComplete;
    expect(summary.value).to.equal('Fix editor');
    type(body, 'Details\n\nRefs #23 #sidebar');
    await ready();
    findAllRelations()[1].click();
    key(body, 'Enter');
    await el.updateComplete;
    expect(body.value).to.equal('Details\n\nRefs #23\nCloses #24');
    find('.signoff-toggle input').click();
    find('.coauthor-wrapper button').click();
    await el.updateComplete;
    type(find<HTMLInputElement>('.coauthor-input'), 'Grace <grace@example.org>');
    find('.coauthor-add-btn').click();
    await el.updateComplete;
    find('.commit-btn').click();
    await waitUntil(() => !!committed);
    expect(committed).to.equal('Fix editor\n\nDetails\n\nRefs #23\nCloses #24\n\nSigned-off-by: Ada <ada@example.org>\nCo-authored-by: Grace <grace@example.org>');
  });

  function findAllRelations() { return el.shadowRoot!.querySelectorAll<HTMLButtonElement>('.issue-relation'); }

  it('keeps a cancelled inline trigger and closes the explicit popover on an outside click', async () => {
    const summary = find<HTMLTextAreaElement>('.summary-input');
    type(summary, 'Keep #');
    await ready();
    key(summary, 'Escape');
    await el.updateComplete;
    expect(summary.value).to.equal('Keep #');
    expect(find('.issue-popover')).to.equal(null);
    open();
    await ready();
    document.body.click();
    await el.updateComplete;
    expect(find('.issue-popover')).to.equal(null);
  });

  it('discards a pending result when switching repositories', async () => {
    pauseList = true;
    open();
    await waitUntil(() => !!finishList);
    expect(find('.issue-status').textContent).to.include('Loading');
    el.repositoryPath = '/b';
    await el.updateComplete;
    finishList!();
    expect(find('.issue-popover')).to.equal(null);
    pauseList = false;
    open();
    await ready();
    expect(find('.issue-repo').textContent).to.include('/b');
  });

  for (const [state, label] of [
    ['offline', 'offline'], ['blocked', 'blocked'], ['unauthenticated', 'Connect an account'],
    ['no-provider', 'No supported'], ['error', 'Temporary failure'], ['empty', 'No matching'],
  ] as const) {
    it(`shows the ${state} state`, async () => {
      if (state === 'offline') settingsStore.getState().setOfflineMode(true);
      else if (state === 'blocked') settingsStore.getState().setRemoteAllowlist(['example.org']);
      else mode = state;
      open();
      await waitUntil(() => !!find('.issue-status')?.textContent?.includes(label));
    });
  }
});
