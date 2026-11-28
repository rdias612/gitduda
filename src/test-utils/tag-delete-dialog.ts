import { waitUntil } from '@open-wc/testing';

export async function answerTagDeleteDialog(confirmed: boolean): Promise<string> {
  const getModal = () =>
    document.querySelector('gn-tag-delete-dialog')?.shadowRoot?.querySelector('gn-modal[open]');
  await waitUntil(getModal, 'The tag deletion dialog should open');

  const modal = getModal()!;
  const message = modal.querySelector('.message')!.textContent!.trim();
  const selector = confirmed ? '.btn-danger' : '.btn-secondary';
  modal.querySelector<HTMLButtonElement>(selector)!.click();
  return message;
}
