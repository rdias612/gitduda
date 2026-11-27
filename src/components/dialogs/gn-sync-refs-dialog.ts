import { css, html, LitElement, nothing } from 'lit';
import { customElement, property, state } from 'lit/decorators.js';
import { localized, msg, str } from '@lit/localize';
import { buttonStyles, sharedStyles } from '../../styles/shared-styles.ts';
import { applySyncRefs, getRemotes, previewSyncRefs } from '../../services/git.service.ts';
import { showToast } from '../../services/notification.service.ts';
import type { SyncRefChange, SyncRefKind, SyncRefsPreview } from '../../types/git.types.ts';
import {
  RefLockController,
  releaseRefOp,
  tryAcquireRefOpOrWarn,
} from '../../utils/ref-lock.ts';
import './gn-modal.ts';
import type { GnModal } from './gn-modal.ts';

@customElement('gn-sync-refs-dialog')
@localized()
export class GnSyncRefsDialog extends LitElement {
  static styles = [
    sharedStyles,
    buttonStyles,
    css`
      .content {
        display: grid;
        gap: var(--spacing-md);
        width: 34rem;
        max-width: 100%;
        font-size: var(--font-size-sm);
      }

      label {
        display: grid;
        gap: var(--spacing-xs);
        font-weight: var(--font-weight-medium);
      }

      select {
        min-width: 0;
        background: var(--color-bg-primary);
        color: var(--color-text-primary);
        border: 1px solid var(--color-border);
        border-radius: var(--radius-md);
        padding: var(--spacing-sm);
        font: inherit;
      }

      p, h3, ul {
        margin: 0;
      }

      h3 {
        font-size: var(--font-size-sm);
      }

      ul {
        padding: 0;
        list-style: none;
      }

      li {
        padding: var(--spacing-sm) 0;
        border-bottom: 1px solid var(--color-border);
        overflow-wrap: anywhere;
      }

      .hashes, .muted {
        color: var(--color-text-secondary);
      }

      code {
        font-family: var(--font-family-mono);
      }

      .warning {
        color: var(--color-warning);
      }

      .error {
        color: var(--color-error);
      }
    `,
  ];

  @property({ type: String }) kind: SyncRefKind = 'branches';
  @state() private repoPath?: string;
  @state() private remotes: string[] = [];
  @state() private remote = '';
  @state() private preview?: SyncRefsPreview;
  @state() private replacement = '';
  @state() private loading = false;
  @state() private applying = false;
  @state() private changed = false;
  @state() private error = '';

  private request = 0;
  private dismissWhenIdle = false;
  private lock = new RefLockController(this, () => this.repoPath);

  get pinnedRepositoryPathIfOpen(): string | null {
    return this.repoPath ?? null;
  }

  get operationInFlight(): boolean {
    return this.applying;
  }

  open(path: string): void {
    if (this.repoPath) return;
    this.repoPath = path;
    this.remotes = [];
    this.remote = '';
    this.resetPreview();
    void this.loadRemotes(path);
  }

  close(): void {
    if (this.applying) return;
    this.request++;
    this.repoPath = undefined;
  }

  closeWhenIdle(): void {
    if (this.applying) this.dismissWhenIdle = true;
    else this.close();
  }

  private resetPreview(): void {
    this.preview = undefined;
    this.replacement = '';
    this.error = '';
    this.changed = false;
  }

  private async loadRemotes(path: string): Promise<void> {
    const request = ++this.request;
    this.error = '';
    this.loading = true;
    const result = await getRemotes(path);
    if (request !== this.request) return;
    this.loading = false;
    if (!result.success || !result.data) {
      this.error = result.error?.message ?? msg('Could not load remotes.');
      return;
    }
    this.remotes = result.data.map(({ name }) => name);
    if (this.remotes.length === 1) {
      this.remote = this.remotes[0];
      await this.loadPreview();
    }
  }

  private async loadPreview(): Promise<void> {
    const path = this.repoPath;
    if (!path || !this.remote) return;
    const request = ++this.request;
    this.resetPreview();
    this.loading = true;
    const result = await previewSyncRefs({ path, remote: this.remote, kind: this.kind });
    if (request !== this.request) return;
    this.loading = false;
    if (result.success && result.data) this.preview = result.data;
    else this.error = result.error?.message ?? msg('Could not load the comparison.');
  }

  private get hasChanges(): boolean {
    const preview = this.preview;
    return !!preview && preview.newRefs.length + preview.deletions.length + preview.overwrites.length > 0;
  }

  private get canApply(): boolean {
    const preview = this.preview;
    return !!preview && this.hasChanges && !this.applying && !this.lock.busy
      && !preview.worktreeBusy && (!preview.currentBranchDeleted || !!this.replacement)
      && ![...preview.newRefs, ...preview.deletions, ...preview.overwrites]
        .some((change) => change.worktree || change.protected);
  }

  private async apply(): Promise<void> {
    const path = this.repoPath;
    const preview = this.preview;
    if (!path || !preview || !this.canApply || !tryAcquireRefOpOrWarn(path)) return;
    this.applying = true;
    this.error = '';
    try {
      const result = await applySyncRefs({
        path, remote: this.remote, kind: this.kind, revision: preview.revision,
        replacement: this.replacement || undefined,
      });
      const outcome = result.success ? result.data : undefined;
      if (outcome?.status === 'changed') {
        this.preview = outcome.preview;
        this.replacement = '';
        this.changed = true;
        return;
      }
      // A local Git error can follow a filesystem write. Refresh from disk too.
      this.dispatchEvent(new CustomEvent('refs-synced', { detail: { repositoryPath: path } }));
      if (outcome?.status !== 'applied') {
        this.resetPreview();
        this.error = result.error?.message ?? msg('Could not synchronize references.');
        return;
      }
      showToast(msg('Local references synchronized.'), 'success');
      this.dismissWhenIdle = true;
    } finally {
      this.applying = false;
      releaseRefOp(path);
      if (this.dismissWhenIdle) {
        this.dismissWhenIdle = false;
        this.close();
      }
    }
  }

  private renderSection(title: string, changes: SyncRefChange[]) {
    return html`
      <section>
        <h3>${title} (${changes.length})</h3>
        <ul>
          ${changes.map((change) => html`
            <li>
              <strong>${change.name}</strong>
              ${change.name === this.preview?.currentBranch
                ? html`<span class="muted">· ${msg('Current branch')}</span>` : nothing}
              <div class="hashes">
                ${change.localOid ? html`<div>${msg('Local hash')} <code>${change.localOid}</code></div>` : nothing}
                ${change.remoteOid ? html`<div>${msg('Remote hash')} <code>${change.remoteOid}</code></div>` : nothing}
              </div>
              ${change.outgoingCommits
                ? html`<div class="warning">${msg(str`Local commits leaving this branch (${change.outgoingCommits})`)}</div>`
                : nothing}
              ${change.worktree
                ? html`<div class="error">${msg(str`Branch in use by another worktree at ${change.worktree}`)}</div>`
                : nothing}
              ${change.protected ? html`<div class="error">${msg('Branch protected from deletion.')}</div>` : nothing}
            </li>
          `)}
        </ul>
      </section>
    `;
  }

  private renderPreview(preview: SyncRefsPreview) {
    return html`
      ${!preview.remoteCount
        ? html`<p>${this.kind === 'tags' ? msg('This remote has no tags.') : msg('This remote has no branches.')}</p>`
        : nothing}
      ${!this.hasChanges ? html`<p role="status">${msg('The clone is already synchronized.')}</p>` : html`
        <p class="muted">${msg('These changes apply only to this clone.')}</p>
        ${preview.worktreeBusy
          ? html`<p class="error">${msg('The current branch cannot change while the worktree is unavailable or a Git operation is in progress.')}</p>`
          : nothing}
        ${preview.workingChanges
          ? html`<p class="warning">${msg('Uncommitted changes in the current worktree may be discarded.')}</p>`
          : nothing}
        ${preview.currentBranchDeleted ? this.renderReplacement(preview.replacements) : nothing}
        ${this.renderSection(msg('New'), preview.newRefs)}
        ${this.renderSection(msg('To delete'), preview.deletions)}
        ${this.renderSection(msg('To overwrite'), preview.overwrites)}
      `}
    `;
  }

  private renderReplacement(choices: string[]) {
    if (choices.length === 0) {
      return html`<p class="error">${msg('No branch is available to switch to before deleting the current branch.')}</p>`;
    }
    return html`
      <label>
        ${msg('Switch to another branch before deleting the current branch')}
        <select id="replacement" ?disabled=${this.applying}
          @change=${(event: Event) => { this.replacement = (event.currentTarget as HTMLSelectElement).value; }}>
          <option value="" .selected=${this.replacement === ''}>${msg('Choose a branch')}</option>
          ${choices.map((name) => html`<option .value=${name} .selected=${name === this.replacement}>${name}</option>`)}
        </select>
      </label>
    `;
  }

  render() {
    return html`
      <gn-modal .open=${!!this.repoPath} .showClose=${!this.applying}
        .modalTitle=${this.kind === 'tags' ? msg('Sync Tags with Remote…') : msg('Sync Branches with Remote…')}
        @close=${(event: Event) => {
          event.stopPropagation();
          if (this.applying) (event.currentTarget as GnModal).open = true;
          else this.close();
        }}>
        <div class="content">
          ${this.remotes.length ? html`
            <label>
              ${msg('Remote')}
              <select id="remote" ?disabled=${this.loading || this.applying}
                @change=${(event: Event) => {
                  this.remote = (event.currentTarget as HTMLSelectElement).value;
                  void this.loadPreview();
                }}>
                <option value="" disabled .selected=${this.remote === ''}>${msg('Choose a remote')}</option>
                ${this.remotes.map((name) => html`<option .value=${name} .selected=${name === this.remote}>${name}</option>`)}
              </select>
            </label>
          ` : nothing}
          ${this.loading ? html`<p role="status">${msg('Loading comparison…')}</p>` : nothing}
          ${this.applying ? html`<p role="status">${msg('Synchronizing local references…')}</p>` : nothing}
          ${!this.loading && !this.error && !this.remotes.length
            ? html`<p>${msg('No remotes configured.')}</p>` : nothing}
          ${this.error ? html`
            <p class="error" role="alert">${this.error}</p>
            <div><button class="btn btn-secondary" @click=${() => {
              if (this.remote) void this.loadPreview();
              else if (this.repoPath) void this.loadRemotes(this.repoPath);
            }}>${msg('Retry')}</button></div>
          ` : nothing}
          ${this.changed
            ? html`<p class="warning" role="status">${msg('The comparison changed. Review it before confirming again.')}</p>`
            : nothing}
          ${this.preview ? this.renderPreview(this.preview) : nothing}
        </div>
        <div slot="footer">
          <button class="btn btn-secondary" ?disabled=${this.applying} @click=${this.close}>${msg('Cancel')}</button>
          <button class="btn btn-primary" ?disabled=${!this.canApply} @click=${this.apply}>${msg('Confirm Sync')}</button>
        </div>
      </gn-modal>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'gn-sync-refs-dialog': GnSyncRefsDialog;
  }
}
