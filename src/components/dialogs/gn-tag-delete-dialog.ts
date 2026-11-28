import { localized, msg, str } from '@lit/localize';
import { LitElement, css, html, nothing } from 'lit';
import { customElement, property } from 'lit/decorators.js';
import { buttonStyles, sharedStyles } from '../../styles/shared-styles.ts';
import './gn-modal.ts';

@customElement('gn-tag-delete-dialog')
@localized()
export class GnTagDeleteDialog extends LitElement {
  static styles = [
    sharedStyles,
    buttonStyles,
    css`
      .message {
        max-width: 420px;
        overflow-wrap: anywhere;
      }

      .message p + p {
        margin-top: var(--spacing-sm);
        color: var(--color-text-secondary);
      }

      .btn-danger {
        border: 1px solid var(--color-error);
        background: var(--color-error-bg);
        color: var(--color-text-primary);
      }

      .btn-danger:hover {
        filter: brightness(0.9);
      }
    `,
  ];

  @property({ attribute: false }) tagName = '';
  @property({ attribute: false }) remote?: string;

  private async finish(confirmed: boolean): Promise<void> {
    const modal = this.renderRoot.querySelector('gn-modal');
    if (modal) {
      modal.open = false;
      await modal.updateComplete;
    }
    this.dispatchEvent(new CustomEvent<boolean>('decision', { detail: confirmed }));
  }

  render() {
    const { tagName, remote } = this;
    return html`
      <gn-modal
        open
        .modalTitle=${remote ? msg('Delete Tag on Remote') : msg('Delete Local Tag')}
        .showClose=${false}
        @close=${(event: Event) => {
          event.stopPropagation();
          void this.finish(false);
        }}
      >
        <div class="message">
          <p>
            ${remote
              ? msg(str`Tag "${tagName}" was deleted locally. Delete it from "${remote}" too?`)
              : msg(str`Delete tag "${tagName}" from this repository?`)}
          </p>
          ${remote
            ? html`<p>${msg('If you keep the tag on the remote, a future fetch can restore it locally.')}</p>`
            : nothing}
        </div>
        <button slot="footer" class="btn btn-secondary" @click=${() => this.finish(false)}>
          ${remote ? msg('Keep on Remote') : msg('Cancel')}
        </button>
        <button slot="footer" class="btn btn-danger" @click=${() => this.finish(true)}>
          ${remote ? msg('Delete on Remote') : msg('Delete Locally')}
        </button>
      </gn-modal>
    `;
  }
}

export async function showTagDeleteDialog(tagName: string, remote?: string): Promise<boolean> {
  const dialog = document.createElement('gn-tag-delete-dialog');
  dialog.tagName = tagName;
  dialog.remote = remote;
  const decision = new Promise<boolean>((resolve) => {
    dialog.addEventListener('decision', (event) => resolve((event as CustomEvent<boolean>).detail), {
      once: true,
    });
  });

  try {
    document.body.append(dialog);
    await dialog.updateComplete;
    return await decision;
  } catch {
    return false;
  } finally {
    dialog.remove();
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'gn-tag-delete-dialog': GnTagDeleteDialog;
  }
}
