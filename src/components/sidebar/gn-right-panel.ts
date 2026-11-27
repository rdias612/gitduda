import { LitElement, html, css, nothing } from 'lit';
import { customElement, property, state } from 'lit/decorators.js';
import { sharedStyles } from '../../styles/shared-styles.ts';
import { repositoryStore } from '../../stores/index.ts';
import * as gitService from '../../services/git.service.ts';
import './gn-file-status.ts';
import './gn-commit-panel.ts';
import '../panels/gn-commit-details.ts';
import '../panels/gn-analytics-panel.ts';
import type { Commit, RefInfo, StatusEntry } from '../../types/git.types.ts';

type TabType = 'changes' | 'details' | 'analytics';

const COMMIT_PANEL_HEIGHT_KEY = 'gn-commit-panel-height';
const FILE_STATUS_MIN_HEIGHT = 100; // Matches .file-status-container min-height
const COMMIT_HANDLE_HEIGHT = 4;
const COMMIT_HANDLE_STEP = 16;

/**
 * Right panel container component
 * Contains file status, commit panel, and commit details in a unified tabbed interface
 */
@customElement('gn-right-panel')
export class GnRightPanel extends LitElement {
  static styles = [
    sharedStyles,
    css`
      :host {
        display: flex;
        flex-direction: column;
        height: 100%;
        overflow: hidden;
      }

      .tab-bar {
        display: flex;
        gap: 2px;
        border-bottom: 1px solid var(--color-border);
        background: var(--color-bg-secondary);
        flex-shrink: 0;
        padding: 0 4px;
      }

      .tab {
        flex: 1;
        padding: 6px 12px;
        font-size: var(--font-size-xs);
        font-weight: var(--font-weight-medium);
        color: var(--color-text-secondary);
        background: transparent;
        border: 1px solid transparent;
        border-bottom: none;
        border-radius: var(--radius-sm) var(--radius-sm) 0 0;
        margin-bottom: -1px;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 6px;
        transition: all var(--transition-fast);
        user-select: none;
      }

      .tab:hover {
        color: var(--color-text-primary);
        background: var(--color-bg-hover);
      }

      .tab.active {
        color: var(--color-primary);
        font-weight: var(--font-weight-semibold);
        background: var(--color-bg-primary);
        border-color: var(--color-border);
        border-bottom-color: var(--color-bg-primary);
      }

      .tab .badge {
        font-size: 10px;
        padding: 1px 5px;
        border-radius: var(--radius-full);
        background: var(--color-bg-tertiary);
        color: var(--color-text-muted);
      }

      .tab.active .badge {
        background: var(--color-primary-bg);
        color: var(--color-primary);
      }

      .tab .indicator {
        width: 6px;
        height: 6px;
        border-radius: 50%;
        background: var(--color-warning);
      }

      .tab-content {
        flex: 1;
        display: flex;
        flex-direction: column;
        overflow: hidden;
        min-height: 0;
      }

      .tab-panel {
        display: none;
        flex-direction: column;
        flex: 1;
        overflow: hidden;
        min-height: 0;
      }

      .tab-panel.active {
        display: flex;
      }

      /* Changes panel layout */
      .changes-panel {
        display: flex;
        flex-direction: column;
        flex: 1;
        min-height: 0;
        overflow: hidden;
      }

      .changes-panel.resizing {
        user-select: none;
      }

      .file-status-container {
        flex: 1;
        overflow: auto;
        min-height: 100px;
      }

      .commit-resize-handle {
        height: 4px;
        cursor: row-resize;
        background: transparent;
        transition: background-color var(--transition-fast);
        flex-shrink: 0;
        z-index: 10;
      }

      .commit-resize-handle:hover,
      .commit-resize-handle.dragging {
        background: var(--color-primary);
      }

      .commit-panel-container {
        flex-shrink: 0;
        border-top: 1px solid var(--color-border);
      }

      /* 104px = 100px (.file-status-container min-height) + 4px (.commit-resize-handle) */
      .commit-panel-container.custom-height {
        min-height: min-content;
        box-sizing: border-box;
        max-height: calc(100% - 104px);
      }

      /* Details panel layout */
      .details-panel {
        flex: 1;
        overflow: auto;
      }

      /* Analytics panel layout */
      .analytics-panel {
        flex: 1;
        overflow: auto;
      }

      .placeholder {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 100%;
        padding: var(--spacing-md);
        color: var(--color-text-muted);
        font-size: var(--font-size-sm);
        text-align: center;
      }

      .shortcut-hint {
        font-size: var(--font-size-xs);
        color: var(--color-text-muted);
        padding: 4px 8px;
        background: var(--color-bg-tertiary);
        border-radius: var(--radius-sm);
        margin-left: auto;
      }

      kbd {
        font-family: var(--font-family-mono);
        font-size: 10px;
        padding: 1px 4px;
        background: var(--color-bg-secondary);
        border: 1px solid var(--color-border);
        border-radius: 3px;
      }
    `,
  ];

  @property({ type: Object }) commit: Commit | null = null;
  @property({ type: Array }) refs: RefInfo[] = [];

  @state() private repositoryPath: string | null = null;
  @state() private stagedCount: number = 0;
  @state() private stagedFiles: StatusEntry[] = [];
  @state() private changesCount: number = 0;
  @state() private activeTab: TabType = 'changes';
  @state() private githubOwner: string = '';
  @state() private githubRepo: string = '';
  @state() private hasUnseenChanges: boolean = false;
  @state() private commitPanelHeight: number | null = null;
  @state() private isResizingCommitPanel: boolean = false;

  private unsubscribe?: () => void;
  private previousCommitOid: string | null = null;
  private commitResizeStartY = 0;
  private commitResizeStartHeight = 0;

  connectedCallback(): void {
    super.connectedCallback();
    this.commitPanelHeight = this.loadCommitPanelHeight();
    const initialState = repositoryStore.getState();
    const initialActiveRepo = initialState.getActiveRepository();
    this.repositoryPath = initialActiveRepo?.repository.path ?? null;
    this.stagedFiles = initialActiveRepo?.stagedFiles ?? [];
    this.stagedCount = this.stagedFiles.length;
    if (this.repositoryPath) {
      this.detectGitHubRepo();
    }

    this.unsubscribe = repositoryStore.subscribe((state) => {
      const activeRepo = state.getActiveRepository();
      const newPath = activeRepo?.repository.path ?? null;
      if (newPath !== this.repositoryPath) {
        this.repositoryPath = newPath;
        this.stagedFiles = activeRepo?.stagedFiles ?? [];
        this.stagedCount = this.stagedFiles.length;
        this.changesCount = activeRepo?.status?.length ?? 0;
        this.hasUnseenChanges = false;
        if (newPath) {
          this.detectGitHubRepo();
        } else {
          this.githubOwner = '';
          this.githubRepo = '';
        }
      }
    });

    // Listen for keyboard shortcuts
    this.addEventListener('keydown', this.handleKeyDown);
  }

  private async detectGitHubRepo(): Promise<void> {
    if (!this.repositoryPath) return;
    const repoPath = this.repositoryPath;

    try {
      const result = await gitService.detectGitHubRepo(repoPath);
      if (this.repositoryPath !== repoPath) return;
      if (result.success && result.data) {
        this.githubOwner = result.data.owner;
        this.githubRepo = result.data.repo;
      } else {
        this.githubOwner = '';
        this.githubRepo = '';
      }
    } catch {
      if (this.repositoryPath !== repoPath) return;
      this.githubOwner = '';
      this.githubRepo = '';
    }
  }

  disconnectedCallback(): void {
    super.disconnectedCallback();
    this.unsubscribe?.();
    this.removeEventListener('keydown', this.handleKeyDown);
    document.removeEventListener('mousemove', this.handleCommitResizeMove);
    document.removeEventListener('mouseup', this.handleCommitResizeEnd);
    this.isResizingCommitPanel = false;
  }

  private loadCommitPanelHeight(): number | null {
    try {
      const raw = localStorage.getItem(COMMIT_PANEL_HEIGHT_KEY);
      if (raw === null) return null;
      const saved = Number(raw);
      return Number.isFinite(saved) && saved > 0 ? saved : null;
    } catch {
      // Ignore storage access errors
      return null;
    }
  }

  private saveCommitPanelHeight(height: number): void {
    try {
      localStorage.setItem(COMMIT_PANEL_HEIGHT_KEY, String(height));
    } catch {
      // Ignore storage access errors
    }
  }

  private clampCommitPanelHeight(value: number): number {
    const changesPanel = this.shadowRoot?.querySelector('.changes-panel') as HTMLElement | null;
    if (!changesPanel) return Math.max(0, Math.round(value));
    const maxHeight = Math.max(
      0,
      changesPanel.clientHeight - FILE_STATUS_MIN_HEIGHT - COMMIT_HANDLE_HEIGHT,
    );
    return Math.max(0, Math.round(Math.min(value, maxHeight)));
  }

  private handleCommitResizeStart = (e: MouseEvent): void => {
    const container = this.shadowRoot?.querySelector('.commit-panel-container') as HTMLElement | null;
    if (!container) return;
    e.preventDefault();
    this.isResizingCommitPanel = true;
    this.commitResizeStartY = e.clientY;
    this.commitResizeStartHeight = container.getBoundingClientRect().height;
    document.addEventListener('mousemove', this.handleCommitResizeMove);
    document.addEventListener('mouseup', this.handleCommitResizeEnd);
  };

  private handleCommitResizeMove = (e: MouseEvent): void => {
    if (!this.isResizingCommitPanel) return;
    const rawHeight = this.commitResizeStartHeight + (this.commitResizeStartY - e.clientY);
    this.commitPanelHeight = this.clampCommitPanelHeight(rawHeight);
  };

  private handleCommitResizeEnd = (): void => {
    if (!this.isResizingCommitPanel) return;
    this.isResizingCommitPanel = false;
    document.removeEventListener('mousemove', this.handleCommitResizeMove);
    document.removeEventListener('mouseup', this.handleCommitResizeEnd);
    if (this.commitPanelHeight !== null) {
      this.saveCommitPanelHeight(this.commitPanelHeight);
    }
  };

  private handleCommitResizeReset = (): void => {
    this.commitPanelHeight = null;
    try {
      localStorage.removeItem(COMMIT_PANEL_HEIGHT_KEY);
    } catch {
      // Ignore storage access errors
    }
  };

  private handleCommitResizeKeyDown = (e: KeyboardEvent): void => {
    if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') return;
    const container = this.shadowRoot?.querySelector('.commit-panel-container') as HTMLElement | null;
    if (!container) return;
    e.preventDefault();
    const currentHeight = container.getBoundingClientRect().height;
    const delta = e.key === 'ArrowUp' ? COMMIT_HANDLE_STEP : -COMMIT_HANDLE_STEP;
    const nextHeight = this.clampCommitPanelHeight(currentHeight + delta);
    this.commitPanelHeight = nextHeight;
    this.saveCommitPanelHeight(nextHeight);
  };

  updated(changedProperties: Map<string, unknown>): void {
    // Auto-switch to details tab when a new commit is selected
    if (changedProperties.has('commit')) {
      const newOid = this.commit?.oid ?? null;
      if (newOid && newOid !== this.previousCommitOid) {
        this.activeTab = 'details';
      }
      this.previousCommitOid = newOid;
    }
  }

  private handleKeyDown = (e: KeyboardEvent): void => {
    // Tab switching shortcuts. Ctrl+SHIFT+digit: plain Ctrl+digit belongs to
    // the global repository-tab shortcuts (browser convention), so the panel
    // takes the shifted chord. Matched on e.code because Shift changes e.key
    // ('1' -> '!') on most layouts.
    if (!e.shiftKey || !(e.ctrlKey || e.metaKey)) return;
    if (e.code === 'Digit1') {
      e.preventDefault();
      this.switchTab('changes');
    } else if (e.code === 'Digit2') {
      e.preventDefault();
      this.switchTab('details');
    } else if (e.code === 'Digit3') {
      e.preventDefault();
      this.switchTab('analytics');
    }
  };

  private switchTab(tab: TabType): void {
    this.activeTab = tab;
    if (tab === 'changes') {
      this.hasUnseenChanges = false;
    }
    this.dispatchEvent(new CustomEvent('tab-changed', {
      detail: { tab },
      bubbles: true,
      composed: true,
    }));
  }

  /**
   * Public method to switch to changes tab from outside.
   *
   * `gn-commit-panel` lives in that tab and is the sole listener for
   * `trigger-amend`. Right-clicking a commit in the graph selects it first,
   * which auto-switches this panel to Details — so amend mode was being turned
   * on inside a `.tab-panel` with `display: none` and the gesture looked like
   * it did nothing.
   */
  public showChanges(): void {
    this.switchTab('changes');
  }

  /** Public method to switch to details tab from outside */
  public showDetails(): void {
    this.switchTab('details');
  }

  /** Public method to switch to analytics tab from outside */
  public showAnalytics(): void {
    this.switchTab('analytics');
  }

  render() {
    if (!this.repositoryPath) {
      return html`<div class="placeholder">No repository open</div>`;
    }

    return html`
      <div class="tab-bar">
        <button
          class="tab ${this.activeTab === 'changes' ? 'active' : ''}"
          @click=${() => this.switchTab('changes')}
          title="Working Changes (Ctrl+Shift+1)"
        >
          <span>Changes</span>
          ${this.changesCount > 0 ? html`<span class="badge">${this.changesCount}</span>` : nothing}
          ${this.hasUnseenChanges && this.activeTab !== 'changes' ? html`<span class="indicator"></span>` : nothing}
        </button>
        <button
          class="tab ${this.activeTab === 'details' ? 'active' : ''}"
          @click=${() => this.switchTab('details')}
          title="Commit Details (Ctrl+Shift+2)"
        >
          <span>Details</span>
          ${this.commit ? html`<span class="badge">${this.commit.oid.slice(0, 7)}</span>` : nothing}
        </button>
        <button
          class="tab ${this.activeTab === 'analytics' ? 'active' : ''}"
          @click=${() => this.switchTab('analytics')}
          title="Repository Analytics (Ctrl+Shift+3)"
        >
          <span>Analytics</span>
        </button>
      </div>

      <div class="tab-content">
        <!-- Changes Panel -->
        <div class="tab-panel ${this.activeTab === 'changes' ? 'active' : ''}">
          <div class="changes-panel ${this.isResizingCommitPanel ? 'resizing' : ''}">
            <div class="file-status-container">
              <gn-file-status
                .repositoryPath=${this.repositoryPath}
                @status-changed=${this.handleStatusChanged}
              ></gn-file-status>
            </div>
            <div
              class="commit-resize-handle ${this.isResizingCommitPanel ? 'dragging' : ''}"
              role="separator"
              aria-orientation="horizontal"
              aria-label="Resize commit panel"
              tabindex="0"
              title="Drag to resize the commit panel (double-click to reset)"
              @mousedown=${this.handleCommitResizeStart}
              @dblclick=${this.handleCommitResizeReset}
              @keydown=${this.handleCommitResizeKeyDown}
            ></div>
            <div
              class="commit-panel-container ${this.commitPanelHeight !== null ? 'custom-height' : ''}"
              style=${this.commitPanelHeight !== null ? `height: ${this.commitPanelHeight}px` : nothing}
            >
              <gn-commit-panel
                .repositoryPath=${this.repositoryPath}
                .stagedCount=${this.stagedCount}
                .stagedFiles=${this.stagedFiles}
                @commit-created=${this.handleCommitCreated}
              ></gn-commit-panel>
            </div>
          </div>
        </div>

        <!-- Details Panel -->
        <div class="tab-panel ${this.activeTab === 'details' ? 'active' : ''}">
          <div class="details-panel">
            <gn-commit-details
              .repositoryPath=${this.repositoryPath}
              .commit=${this.commit}
              .refs=${this.refs}
              .githubOwner=${this.githubOwner}
              .githubRepo=${this.githubRepo}
            ></gn-commit-details>
          </div>
        </div>

        <!-- Analytics Panel -->
        <div class="tab-panel ${this.activeTab === 'analytics' ? 'active' : ''}">
          <div class="analytics-panel">
            <gn-analytics-panel
              .repositoryPath=${this.repositoryPath}
              .active=${this.activeTab === 'analytics'}
            ></gn-analytics-panel>
          </div>
        </div>
      </div>
    `;
  }

  private handleStatusChanged(
    e: CustomEvent<{ stagedCount: number; totalCount?: number; stagedFiles?: StatusEntry[] }>,
  ): void {
    this.stagedCount = e.detail.stagedCount;
    if (e.detail.stagedFiles !== undefined || this.stagedCount === 0) {
      this.stagedFiles = e.detail.stagedFiles ?? [];
    }
    if (e.detail.totalCount !== undefined) {
      const previousCount = this.changesCount;
      this.changesCount = e.detail.totalCount;
      // Show indicator if changes happened while on details tab
      if (this.activeTab === 'details' && this.changesCount > previousCount) {
        this.hasUnseenChanges = true;
      }
    }
  }

  /** Forward the repo the commit ran in (captured pre-await by the commit
   * panel) so the host pins the refresh to it, not whichever tab is active if
   * the user switched mid-commit. */
  private handleCommitCreated(e?: Event): void {
    const repoPath = (e as CustomEvent<{ repositoryPath?: string }> | undefined)?.detail
      ?.repositoryPath;
    this.dispatchEvent(
      new CustomEvent('repository-changed', {
        detail: repoPath ? { repositoryPath: repoPath } : undefined,
        bubbles: true,
        composed: true,
      })
    );
    window.dispatchEvent(
      new CustomEvent('repository-refresh', {
        detail: { ...(repoPath ? { repoPath } : {}), source: 'sidebar-panel' },
      })
    );
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'gn-right-panel': GnRightPanel;
  }
}
