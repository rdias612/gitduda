import { expect, fixture, html } from '@open-wc/testing';
import { LitElement, html as litHtml } from 'lit';
import { customElement } from 'lit/decorators.js';
import { deepActiveElement, containsDeepActiveElement } from '../focus.ts';

@customElement('focus-test-host')
class FocusTestHost extends LitElement {
  render() {
    return litHtml`
      <input id="shadow-input" />
      <slot></slot>
    `;
  }
}

@customElement('focus-test-child')
class FocusTestChild extends LitElement {
  render() {
    return litHtml`<input id="child-shadow-input" />`;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'focus-test-host': FocusTestHost;
    'focus-test-child': FocusTestChild;
  }
}

describe('focus utils', () => {
  afterEach(() => {
    (document.activeElement as HTMLElement | null)?.blur?.();
  });

  it('deepActiveElement returns null when nothing is focused', () => {
    (document.activeElement as HTMLElement | null)?.blur?.();
    expect(deepActiveElement()).to.equal(null);
  });

  it('deepActiveElement walks through shadow roots to the real target', async () => {
    const el = await fixture<FocusTestHost>(html`<focus-test-host></focus-test-host>`);
    await el.updateComplete;

    const input = el.shadowRoot!.querySelector<HTMLInputElement>('#shadow-input')!;
    input.focus();

    // document.activeElement only names the host — the helper must go deeper.
    expect(document.activeElement).to.equal(el);
    expect(deepActiveElement()).to.equal(input);
  });

  it('containsDeepActiveElement is true for a field in the host shadow root', async () => {
    const el = await fixture<FocusTestHost>(html`<focus-test-host></focus-test-host>`);
    await el.updateComplete;

    expect(containsDeepActiveElement(el)).to.equal(false);

    el.shadowRoot!.querySelector<HTMLInputElement>('#shadow-input')!.focus();
    expect(containsDeepActiveElement(el)).to.equal(true);
  });

  it('containsDeepActiveElement is true for slotted light-DOM content', async () => {
    const el = await fixture<FocusTestHost>(
      html`<focus-test-host><input id="light-input" /></focus-test-host>`,
    );
    await el.updateComplete;

    const light = el.querySelector<HTMLInputElement>('#light-input')!;
    light.focus();

    expect(deepActiveElement()).to.equal(light);
    expect(containsDeepActiveElement(el)).to.equal(true);
  });

  it('containsDeepActiveElement is false when focus sits in a sibling element', async () => {
    const wrap = await fixture<HTMLDivElement>(html`
      <div>
        <focus-test-host></focus-test-host>
        <input id="outside" />
      </div>
    `);
    const host = wrap.querySelector<FocusTestHost>('focus-test-host')!;
    await host.updateComplete;

    wrap.querySelector<HTMLInputElement>('#outside')!.focus();
    expect(containsDeepActiveElement(host)).to.equal(false);
  });

  it('containsDeepActiveElement walks across slotted child shadow roots', async () => {
    const el = await fixture<FocusTestHost>(
      html`<focus-test-host><focus-test-child></focus-test-child></focus-test-host>`,
    );
    await el.updateComplete;
    const child = el.querySelector<FocusTestChild>('focus-test-child')!;
    await child.updateComplete;

    const childInput = child.shadowRoot!.querySelector<HTMLInputElement>('#child-shadow-input')!;
    childInput.focus();

    expect(deepActiveElement()).to.equal(childInput);
    expect(containsDeepActiveElement(el)).to.equal(true);
  });

  it('containsDeepActiveElement is false for a sibling element with its own shadow root', async () => {
    const wrap = await fixture<HTMLDivElement>(html`
      <div>
        <focus-test-host></focus-test-host>
        <focus-test-child></focus-test-child>
      </div>
    `);
    const host = wrap.querySelector<FocusTestHost>('focus-test-host')!;
    const sibling = wrap.querySelector<FocusTestChild>('focus-test-child')!;
    await host.updateComplete;
    await sibling.updateComplete;

    sibling.shadowRoot!.querySelector<HTMLInputElement>('#child-shadow-input')!.focus();
    expect(containsDeepActiveElement(host)).to.equal(false);
  });
});
