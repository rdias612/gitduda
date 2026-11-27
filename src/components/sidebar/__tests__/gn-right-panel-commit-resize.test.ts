/**
 * Tests for the commit panel vertical resize handle in gn-right-panel.
 */

// ── Tauri mock (must be set before any imports) ────────────────────────────
let cbId = 0;
(globalThis as Record<string, unknown>).__TAURI_INTERNALS__ = {
  invoke: () => Promise.resolve(null),
  transformCallback: () => cbId++,
};

// ── Imports (after Tauri mock) ─────────────────────────────────────────────
import { expect, fixture, html } from '@open-wc/testing';
import type { GnRightPanel } from '../gn-right-panel.ts';
import type { GnCommitPanel } from '../gn-commit-panel.ts';
import '../gn-right-panel.ts';
import { repositoryStore } from '../../../stores/index.ts';
import type { Repository } from '../../../types/git.types.ts';

const STORAGE_KEY = 'gn-commit-panel-height';

function mockRepo(path: string): Repository {
  return {
    path,
    name: 'test-repo',
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

async function mountPanel(): Promise<{
  panel: GnRightPanel;
  handle: HTMLElement;
  fileStatusContainer: HTMLElement;
  commitContainer: HTMLElement;
  commitPanel: GnCommitPanel;
}> {
  const wrapper = await fixture<HTMLDivElement>(html`
    <div style="height: 650px; width: 350px;">
      <gn-right-panel></gn-right-panel>
    </div>
  `);
  const panel = wrapper.querySelector('gn-right-panel') as GnRightPanel;
  await panel.updateComplete;
  const commitPanel = panel.shadowRoot!.querySelector('gn-commit-panel') as GnCommitPanel;
  await commitPanel.updateComplete;
  await panel.updateComplete;

  return {
    panel,
    handle: panel.shadowRoot!.querySelector('.commit-resize-handle') as HTMLElement,
    fileStatusContainer: panel.shadowRoot!.querySelector('.file-status-container') as HTMLElement,
    commitContainer: panel.shadowRoot!.querySelector('.commit-panel-container') as HTMLElement,
    commitPanel,
  };
}

describe('gn-right-panel commit panel resize', () => {
  beforeEach(() => {
    localStorage.removeItem(STORAGE_KEY);
    repositoryStore.getState().reset();
    repositoryStore.getState().addRepository(mockRepo('/test/repo'));
  });

  afterEach(() => {
    localStorage.removeItem(STORAGE_KEY);
  });

  it('renders a horizontal separator handle above the commit panel', async () => {
    const { handle } = await mountPanel();

    expect(handle).to.exist;
    expect(handle.getAttribute('role')).to.equal('separator');
    expect(handle.getAttribute('aria-orientation')).to.equal('horizontal');
    expect(handle.getAttribute('tabindex')).to.equal('0');
  });

  it('resizes commit panel upward on drag and persists height on mouseup', async () => {
    const { panel, handle, commitContainer } = await mountPanel();
    const initialHeight = commitContainer.getBoundingClientRect().height;
    const startY = 400;
    const dragDelta = 80;

    handle.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true, clientY: startY }));
    document.dispatchEvent(new MouseEvent('mousemove', { clientY: startY - dragDelta }));
    await panel.updateComplete;

    const resizedHeight = commitContainer.getBoundingClientRect().height;
    expect(resizedHeight).to.be.closeTo(initialHeight + dragDelta, 1);

    document.dispatchEvent(new MouseEvent('mouseup', { clientY: startY - dragDelta }));
    await panel.updateComplete;

    const saved = Number(localStorage.getItem(STORAGE_KEY));
    expect(saved).to.equal(Math.round(initialHeight + dragDelta));
  });

  it('clamps maximum height so file status container keeps at least 100px', async () => {
    const { panel, handle, fileStatusContainer, commitContainer } = await mountPanel();

    handle.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true, clientY: 400 }));
    document.dispatchEvent(new MouseEvent('mousemove', { clientY: -500 }));
    await panel.updateComplete;
    document.dispatchEvent(new MouseEvent('mouseup', { clientY: -500 }));
    await panel.updateComplete;

    expect(fileStatusContainer.getBoundingClientRect().height).to.be.at.least(100);
    expect(commitContainer.getBoundingClientRect().height).to.be.greaterThan(0);
  });

  it('keeps commit button inside container bounds at minimum height', async () => {
    const { panel, handle, commitContainer, commitPanel } = await mountPanel();

    handle.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true, clientY: 300 }));
    document.dispatchEvent(new MouseEvent('mousemove', { clientY: 1200 }));
    await panel.updateComplete;
    document.dispatchEvent(new MouseEvent('mouseup', { clientY: 1200 }));
    await panel.updateComplete;
    await commitPanel.updateComplete;

    const containerRect = commitContainer.getBoundingClientRect();
    const commitBtn = commitPanel.shadowRoot!.querySelector('.commit-btn') as HTMLElement;
    expect(commitBtn).to.exist;
    const btnRect = commitBtn.getBoundingClientRect();

    expect(btnRect.height).to.be.greaterThan(0);
    expect(btnRect.top).to.be.at.least(containerRect.top);
    expect(btnRect.bottom).to.be.at.most(containerRect.bottom + 0.5);
  });

  it('resets height to automatic and removes storage key on dblclick', async () => {
    localStorage.setItem(STORAGE_KEY, '360');
    const { panel, handle, commitContainer } = await mountPanel();

    expect(commitContainer.getBoundingClientRect().height).to.be.closeTo(360, 1);

    handle.dispatchEvent(new MouseEvent('dblclick', { bubbles: true, cancelable: true }));
    await panel.updateComplete;

    expect(localStorage.getItem(STORAGE_KEY)).to.be.null;
    expect(commitContainer.classList.contains('custom-height')).to.be.false;
    expect(commitContainer.getBoundingClientRect().height).to.be.lessThan(360);
  });

  it('increases height by 16px on ArrowUp and persists it', async () => {
    const { panel, handle, commitContainer } = await mountPanel();
    const initialHeight = commitContainer.getBoundingClientRect().height;

    handle.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true, cancelable: true }));
    await panel.updateComplete;

    const updatedHeight = commitContainer.getBoundingClientRect().height;
    expect(updatedHeight).to.be.closeTo(initialHeight + 16, 1);
    expect(Number(localStorage.getItem(STORAGE_KEY))).to.equal(Math.round(initialHeight + 16));
  });

  it('ignores invalid localStorage values on mount', async () => {
    for (const invalid of ['not-a-number', '-40', '0', 'Infinity']) {
      localStorage.setItem(STORAGE_KEY, invalid);
      const { commitContainer } = await mountPanel();
      expect(commitContainer.classList.contains('custom-height')).to.be.false;
    }
  });

  it('applies saved height from localStorage on mount', async () => {
    localStorage.setItem(STORAGE_KEY, '340');
    const { commitContainer } = await mountPanel();

    expect(commitContainer.classList.contains('custom-height')).to.be.true;
    expect(commitContainer.getBoundingClientRect().height).to.be.closeTo(340, 1);
  });
});
