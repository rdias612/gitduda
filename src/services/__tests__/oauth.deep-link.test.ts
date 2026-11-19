/**
 * OAuth deep-link scheme recognition.
 */

import { expect } from '@open-wc/testing';

// Same environment shims as oauth.service.test.ts — the module reads
// import.meta.env and the Tauri globals when it is evaluated.
(globalThis as unknown as { import: { meta: { env: Record<string, string> } } }).import = {
  meta: { env: { DEV: 'true', MODE: 'test' } },
};
if (typeof import.meta === 'object' && !import.meta.env) {
  (import.meta as unknown as { env: Record<string, string> }).env = { DEV: 'true', MODE: 'test' };
}
(globalThis as unknown as { __TAURI_INTERNALS__: { invoke: () => Promise<unknown> } }).__TAURI_INTERNALS__ = {
  invoke: () => Promise.resolve(null),
};

import { OAUTH_DEEP_LINK_PREFIXES, oauthDeepLinkProvider } from '../oauth.service.ts';

describe('oauthDeepLinkProvider', () => {
  it('declares the gitduda://oauth/ scheme prefix', () => {
    expect(OAUTH_DEEP_LINK_PREFIXES).to.deep.equal(['gitduda://oauth/']);
  });

  it('extracts the provider from a gitduda:// callback', () => {
    expect(oauthDeepLinkProvider('gitduda://oauth/gitlab/callback?code=abc&state=xyz')).to.equal(
      'gitlab'
    );
  });

  it('handles a provider segment with no trailing path', () => {
    expect(oauthDeepLinkProvider('gitduda://oauth/github?code=1')).to.equal('github');
    expect(oauthDeepLinkProvider('gitduda://oauth/github')).to.equal('github');
  });

  it('rejects non-OAuth and foreign-scheme links', () => {
    expect(oauthDeepLinkProvider('gitduda://open?repo=x')).to.be.null;
    expect(oauthDeepLinkProvider('https://example.com/oauth/gitlab/callback')).to.be.null;
    expect(oauthDeepLinkProvider('other://oauth/gitlab/callback')).to.be.null;
    expect(oauthDeepLinkProvider('')).to.be.null;
  });

  it('rejects an OAuth link with an empty provider segment', () => {
    expect(oauthDeepLinkProvider('gitduda://oauth/')).to.be.null;
    expect(oauthDeepLinkProvider('gitduda://oauth//callback')).to.be.null;
  });
});
