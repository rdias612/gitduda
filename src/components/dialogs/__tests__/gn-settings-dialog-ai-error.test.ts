/**
 * Settings Dialog AI error reporting tests.
 */

import { expect, fixture, html } from '@open-wc/testing';

type MockInvoke = (command: string, args?: unknown) => Promise<unknown>;

let providersResponse: unknown[] = [];

const mockInvoke: MockInvoke = async (command: string) => {
  if (command === 'plugin:notification|is_permission_granted') return false;

  switch (command) {
    case 'get_ai_providers':
      return providersResponse;
    case 'get_app_version':
      return '0.1.0';
    case 'get_settings':
      return {};
    case 'get_mcp_status':
      return { servers: [], totalTools: 0 };
    case 'get_available_diff_tools':
      return [];
    case 'get_merge_tool_info':
      return null;
    case 'get_graph_color_schemes':
      return [];
    default:
      return null;
  }
};

(globalThis as unknown as { __TAURI_INTERNALS__: { invoke: MockInvoke } }).__TAURI_INTERNALS__ = {
  invoke: mockInvoke,
};

import '../gn-settings-dialog.ts';
import type { GnSettingsDialog } from '../gn-settings-dialog.ts';

async function createComponent(): Promise<GnSettingsDialog> {
  const el = await fixture<GnSettingsDialog>(html`<gn-settings-dialog></gn-settings-dialog>`);
  await el.updateComplete;
  return el;
}

describe('gn-settings-dialog AI error reporting', () => {
  beforeEach(() => {
    providersResponse = [];
  });

  it('shows the keyring failure reported by the backend when providers load', async () => {
    providersResponse = [
      {
        providerType: 'open_ai',
        name: 'OpenAI',
        available: false,
        probed: false,
        requiresApiKey: true,
        hasApiKey: false,
        keyError: 'Failed to read the api-key:open_ai API key from the OS keyring: locked',
        endpoint: '',
        models: [],
        selectedModel: null,
      },
    ];
    const el = await createComponent();
    await (el as unknown as { loadAiProviders: () => Promise<void> }).loadAiProviders();
    await el.updateComplete;

    expect((el as unknown as { aiError: string | null }).aiError).to.contain('OS keyring');
  });
});
