/**
 * Shadow-DOM aware focus helpers.
 *
 * `document.activeElement` only ever names the outermost custom element, so a
 * focused field inside a component's shadow root reads as that component. Every
 * "is the user already typing in here?" check has to walk the shadow roots down
 * to the real target first.
 */

/**
 * The deepest focused element, following shadow roots.
 * Returns `null` when nothing meaningful is focused (i.e. focus is on `body`).
 */
export function deepActiveElement(): Element | null {
  let el: Element | null = document.activeElement;
  while (el?.shadowRoot?.activeElement) {
    el = el.shadowRoot.activeElement;
  }
  return el && el !== document.body ? el : null;
}

/**
 * True when focus already sits inside `host` — in its light DOM (slotted
 * content) or its own shadow root. Use this to guard any deferred `focus()`
 * so it cannot yank the caret out of a field the user has already moved to.
 */
export function containsDeepActiveElement(host: Element): boolean {
  let node: Node | null = deepActiveElement();
  while (node) {
    if (host.contains(node) || (host.shadowRoot?.contains(node) ?? false)) {
      return true;
    }
    const root = node.getRootNode();
    node = root instanceof ShadowRoot ? root.host : null;
  }
  return false;
}
