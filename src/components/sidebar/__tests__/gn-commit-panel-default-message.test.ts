/**
 * Tests for gn-commit-panel default commit message behavior (Issue #4).
 *
 * Covers auto-filling default messages from stagedFiles, dynamic updates,
 * clearing on unstage-all, preserving manual/amend/template/history/AI edits,
 * repository draftCache isolation, and reset after a successful commit.
 */

// ── Tauri mock (must be set before any imports) ────────────────────────────
type MockInvoke = (command: string, args?: unknown) => Promise<unknown>;

let cbId = 0;
let mockInvoke: MockInvoke = () => Promise.resolve(null);
const invokeHistory: Array<{ command: string; args?: unknown }> = [];

(globalThis as Record<string, unknown>).__TAURI_INTERNALS__ = {
  invoke: (command: string, args?: unknown) => {
    invokeHistory.push({ command, args });
    return mockInvoke(command, args);
  },
  transformCallback: () => cbId++,
};

// Mock the Tauri event plugin internals (used by @tauri-apps/api/event)
(globalThis as Record<string, unknown>).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
  convertCallback: (callback: unknown, once: boolean) => {
    void once;
    void callback;
    return 0;
  },
  unregisterListener: (_event: string, _eventId: number) => {},
};

// ── Imports (after Tauri mock) ─────────────────────────────────────────────
import { expect, fixture, html } from '@open-wc/testing';
import { repositoryStore } from '../../../stores/repository.store.ts';
import type { FileStatus, StatusEntry } from '../../../types/git.types.ts';
import type { CommitTemplate } from '../../../services/git.service.ts';
import type { GnCommitPanel } from '../gn-commit-panel.ts';
import '../gn-commit-panel.ts';

// ── Test data & helpers ────────────────────────────────────────────────────
const REPO_PATH = '/test/repo';
const REPO_PATH_A = '/test/repo-a';
const REPO_PATH_B = '/test/repo-b';

interface MockOptions {
  templates?: CommitTemplate[];
  aiAvailable?: boolean;
  generatedMessage?: { summary: string; body: string | null };
}

function stagedFile(path: string, status: FileStatus): StatusEntry {
  return {
    path,
    status,
    isStaged: true,
    isConflicted: false,
  };
}

function setupDefaultMocks(options: MockOptions = {}): void {
  const templates = options.templates ?? [];
  const aiAvailable = options.aiAvailable ?? false;
  const generatedMessage = options.generatedMessage ?? {
    summary: 'AI generated summary',
    body: 'AI generated body',
  };

  mockInvoke = async (command: string) => {
    switch (command) {
      case 'list_templates':
        return templates;
      case 'get_conventional_types':
        return [
          { typeName: 'feat', description: 'A new feature', emoji: '✨' },
          { typeName: 'fix', description: 'A bug fix', emoji: '🐛' },
        ];
      case 'get_commit_template':
        return null;
      case 'get_user_identity':
        return { name: 'Test User', email: 'test@example.com' };
      case 'is_ai_available':
        return aiAvailable;
      case 'get_ai_unavailable_reason':
        return { reason: '' };
      case 'generate_commit_message':
        return generatedMessage;
      case 'get_commit_history':
        return [
          {
            oid: 'abc123def456',
            shortId: 'abc123d',
            message: 'Last commit message\n\nSome body text',
            summary: 'Last commit message',
            body: 'Some body text',
            author: { name: 'Test', email: 'test@test.com', timestamp: 0 },
            committer: { name: 'Test', email: 'test@test.com', timestamp: 0 },
            parentIds: [],
            timestamp: 0,
          },
        ];
      case 'create_commit':
        return { shortId: 'new123', oid: 'new123456', summary: 'test commit' };
      default:
        return null;
    }
  };
}

function addTestRepository(path: string, name: string): void {
  repositoryStore.getState().addRepository({
    path,
    name,
    isValid: true,
    isBare: false,
    headRef: null,
    detachedHeadOid: null,
    state: 'clean',
    isShallow: false,
    isPartialClone: false,
    cloneFilter: null,
  });
}

function setupStore(): void {
  addTestRepository(REPO_PATH, 'test-repo');
  repositoryStore.getState().setCurrentBranch({
    name: 'main',
    shorthand: 'main',
    isHead: true,
    isRemote: false,
    upstream: null,
    targetOid: 'abc123',
    isStale: false,
  });
}

async function flushAsync(ms = 60): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, ms));
}

async function renderCommitPanel(
  stagedFiles: StatusEntry[] = [],
  repositoryPath = REPO_PATH
): Promise<GnCommitPanel> {
  const el = await fixture<GnCommitPanel>(
    html`<gn-commit-panel
      .repositoryPath=${repositoryPath}
      .stagedCount=${stagedFiles.length}
      .stagedFiles=${stagedFiles}
    ></gn-commit-panel>`
  );
  await flushAsync(80);
  await el.updateComplete;
  return el;
}

function getSummaryInput(el: GnCommitPanel): HTMLTextAreaElement {
  return el.shadowRoot!.querySelector('.summary-input') as HTMLTextAreaElement;
}

function getDescriptionInput(el: GnCommitPanel): HTMLTextAreaElement {
  return el.shadowRoot!.querySelector('.description-input') as HTMLTextAreaElement;
}

async function setStagedFiles(el: GnCommitPanel, files: StatusEntry[]): Promise<void> {
  el.stagedFiles = files;
  el.stagedCount = files.length;
  await el.updateComplete;
}

// ── Tests ──────────────────────────────────────────────────────────────────
describe('gn-commit-panel default commit message', () => {
  beforeEach(() => {
    invokeHistory.length = 0;
    localStorage.removeItem('gitnado-commit-history');
    setupDefaultMocks();
    setupStore();
  });

  afterEach(() => {
    localStorage.removeItem('gitnado-commit-history');
    repositoryStore.getState().reset();
  });

  // ── 1. Single staged file ────────────────────────────────────────────────
  describe('single staged file', () => {
    const cases: Array<{ status: FileStatus; path: string; expectedSummary: string }> = [
      { status: 'new', path: 'src/new-file.ts', expectedSummary: 'add file src/new-file.ts' },
      { status: 'modified', path: 'src/app.ts', expectedSummary: 'modified file src/app.ts' },
      { status: 'deleted', path: 'src/legacy.ts', expectedSummary: 'deleted file src/legacy.ts' },
      { status: 'renamed', path: 'src/renamed.ts', expectedSummary: 'renamed file src/renamed.ts' },
    ];

    for (const { status, path, expectedSummary } of cases) {
      it(`fills summary for a single "${status}" staged file and keeps description empty`, async () => {
        const el = await renderCommitPanel([stagedFile(path, status)]);

        expect(getSummaryInput(el).value).to.equal(expectedSummary);
        expect(getDescriptionInput(el).value).to.equal('');
      });
    }
  });

  // ── 2. Multiple staged files ─────────────────────────────────────────────
  describe('multiple staged files', () => {
    it('fills summary with "multiple changes" and lists each file line in description', async () => {
      const files = [
        stagedFile('src/new.ts', 'new'),
        stagedFile('src/app.ts', 'modified'),
        stagedFile('src/old.ts', 'deleted'),
        stagedFile('src/moved.ts', 'renamed'),
      ];
      const el = await renderCommitPanel(files);

      expect(getSummaryInput(el).value).to.equal('multiple changes');
      expect(getDescriptionInput(el).value).to.equal(
        [
          'add file src/new.ts',
          'modified file src/app.ts',
          'deleted file src/old.ts',
          'renamed file src/moved.ts',
        ].join('\n')
      );
    });
  });

  // ── 3. Dynamic updates while auto-generated ──────────────────────────────
  describe('dynamic updates while message is still default', () => {
    it('updates message dynamically across 1 -> 2 -> 1 staged files', async () => {
      const fileA = stagedFile('src/a.ts', 'new');
      const fileB = stagedFile('src/b.ts', 'modified');

      const el = await renderCommitPanel([fileA]);
      expect(getSummaryInput(el).value).to.equal('add file src/a.ts');
      expect(getDescriptionInput(el).value).to.equal('');

      await setStagedFiles(el, [fileA, fileB]);
      expect(getSummaryInput(el).value).to.equal('multiple changes');
      expect(getDescriptionInput(el).value).to.equal('add file src/a.ts\nmodified file src/b.ts');

      await setStagedFiles(el, [fileB]);
      expect(getSummaryInput(el).value).to.equal('modified file src/b.ts');
      expect(getDescriptionInput(el).value).to.equal('');
    });
  });

  // ── 4. Unstaging all files ───────────────────────────────────────────────
  describe('unstaging all files', () => {
    it('clears summary and description when stagedFiles becomes empty and message is default', async () => {
      const el = await renderCommitPanel([
        stagedFile('src/a.ts', 'new'),
        stagedFile('src/b.ts', 'modified'),
      ]);

      expect(getSummaryInput(el).value).to.equal('multiple changes');
      expect(getDescriptionInput(el).value).to.equal('add file src/a.ts\nmodified file src/b.ts');

      await setStagedFiles(el, []);
      expect(getSummaryInput(el).value).to.equal('');
      expect(getDescriptionInput(el).value).to.equal('');
    });
  });

  // ── 5. Manual edits protect against overwrite ────────────────────────────
  describe('manual edits', () => {
    it('does not overwrite or clear after manual summary edit when stagedFiles changes', async () => {
      const fileA = stagedFile('src/a.ts', 'new');
      const fileB = stagedFile('src/b.ts', 'modified');
      const el = await renderCommitPanel([fileA]);

      const summaryInput = getSummaryInput(el);
      summaryInput.value = 'Custom user summary';
      summaryInput.dispatchEvent(new Event('input'));
      await el.updateComplete;

      await setStagedFiles(el, [fileA, fileB]);
      expect(getSummaryInput(el).value).to.equal('Custom user summary');
      expect(getDescriptionInput(el).value).to.equal('');

      await setStagedFiles(el, []);
      expect(getSummaryInput(el).value).to.equal('Custom user summary');
      expect(getDescriptionInput(el).value).to.equal('');
    });

    it('does not overwrite or clear after manual description edit when stagedFiles changes', async () => {
      const fileA = stagedFile('src/a.ts', 'new');
      const fileB = stagedFile('src/b.ts', 'modified');
      const el = await renderCommitPanel([fileA]);

      const descInput = getDescriptionInput(el);
      descInput.value = 'Custom user details';
      descInput.dispatchEvent(new Event('input'));
      await el.updateComplete;

      await setStagedFiles(el, [fileA, fileB]);
      expect(getSummaryInput(el).value).to.equal('add file src/a.ts');
      expect(getDescriptionInput(el).value).to.equal('Custom user details');

      await setStagedFiles(el, []);
      expect(getSummaryInput(el).value).to.equal('add file src/a.ts');
      expect(getDescriptionInput(el).value).to.equal('Custom user details');
    });
  });

  // ── 6. Amend, template, history, and AI generation ───────────────────────
  describe('amend, template, history, and AI generation precedence', () => {
    it('does not overwrite amend message when stagedFiles changes', async () => {
      const fileA = stagedFile('src/a.ts', 'new');
      const fileB = stagedFile('src/b.ts', 'modified');
      const el = await renderCommitPanel([fileA]);

      const amendCheckbox = el.shadowRoot!.querySelector('.amend-toggle input') as HTMLInputElement;
      amendCheckbox.checked = true;
      amendCheckbox.dispatchEvent(new Event('change'));
      await flushAsync();
      await el.updateComplete;

      expect(getSummaryInput(el).value).to.equal('Last commit message');
      expect(getDescriptionInput(el).value).to.equal('Some body text');

      await setStagedFiles(el, [fileA, fileB]);
      expect(getSummaryInput(el).value).to.equal('Last commit message');
      expect(getDescriptionInput(el).value).to.equal('Some body text');

      await setStagedFiles(el, []);
      expect(getSummaryInput(el).value).to.equal('Last commit message');
      expect(getDescriptionInput(el).value).to.equal('Some body text');
    });

    it('does not overwrite template message when stagedFiles changes', async () => {
      setupDefaultMocks({
        templates: [
          {
            id: 'tpl-1',
            name: 'Feature template',
            content: 'Template summary\nTemplate body',
            isConventional: false,
            createdAt: 1,
          },
        ],
      });

      const fileA = stagedFile('src/a.ts', 'new');
      const fileB = stagedFile('src/b.ts', 'modified');
      const el = await renderCommitPanel([fileA]);

      const templateSelect = el.shadowRoot!.querySelector('.template-select') as HTMLSelectElement;
      expect(templateSelect).to.exist;
      templateSelect.value = 'tpl-1';
      templateSelect.dispatchEvent(new Event('change'));
      await el.updateComplete;

      expect(getSummaryInput(el).value).to.equal('Template summary');
      expect(getDescriptionInput(el).value).to.equal('Template body');

      await setStagedFiles(el, [fileA, fileB]);
      expect(getSummaryInput(el).value).to.equal('Template summary');
      expect(getDescriptionInput(el).value).to.equal('Template body');

      await setStagedFiles(el, []);
      expect(getSummaryInput(el).value).to.equal('Template summary');
      expect(getDescriptionInput(el).value).to.equal('Template body');
    });

    it('does not overwrite history message when stagedFiles changes', async () => {
      localStorage.setItem(
        'gitnado-commit-history',
        JSON.stringify(['Previous commit summary\n\nPrevious commit body'])
      );

      const fileA = stagedFile('src/a.ts', 'new');
      const fileB = stagedFile('src/b.ts', 'modified');
      const el = await renderCommitPanel([fileA]);

      const historyBtn = el.shadowRoot!.querySelector('.history-btn') as HTMLButtonElement;
      historyBtn.click();
      await el.updateComplete;

      const historyItem = el.shadowRoot!.querySelector('.history-item') as HTMLButtonElement;
      expect(historyItem).to.exist;
      historyItem.click();
      await el.updateComplete;

      expect(getSummaryInput(el).value).to.equal('Previous commit summary');
      expect(getDescriptionInput(el).value).to.equal('Previous commit body');

      await setStagedFiles(el, [fileA, fileB]);
      expect(getSummaryInput(el).value).to.equal('Previous commit summary');
      expect(getDescriptionInput(el).value).to.equal('Previous commit body');

      await setStagedFiles(el, []);
      expect(getSummaryInput(el).value).to.equal('Previous commit summary');
      expect(getDescriptionInput(el).value).to.equal('Previous commit body');
    });

    it('does not overwrite AI-generated message when stagedFiles changes', async () => {
      setupDefaultMocks({
        aiAvailable: true,
        generatedMessage: {
          summary: 'feat: generate commit message',
          body: 'Generated from staged diff',
        },
      });

      const fileA = stagedFile('src/a.ts', 'new');
      const fileB = stagedFile('src/b.ts', 'modified');
      const el = await renderCommitPanel([fileA]);

      const generateBtn = el.shadowRoot!.querySelector('.generate-btn') as HTMLButtonElement;
      expect(generateBtn).to.exist;
      expect(generateBtn.disabled).to.be.false;
      generateBtn.click();
      await flushAsync();
      await el.updateComplete;

      expect(getSummaryInput(el).value).to.equal('feat: generate commit message');
      expect(getDescriptionInput(el).value).to.equal('Generated from staged diff');

      await setStagedFiles(el, [fileA, fileB]);
      expect(getSummaryInput(el).value).to.equal('feat: generate commit message');
      expect(getDescriptionInput(el).value).to.equal('Generated from staged diff');

      await setStagedFiles(el, []);
      expect(getSummaryInput(el).value).to.equal('feat: generate commit message');
      expect(getDescriptionInput(el).value).to.equal('Generated from staged diff');
    });
  });

  // ── 7. Repository switching & draftCache isolation ───────────────────────
  describe('repository switching and draftCache', () => {
    it('preserves draft and auto-message flags per repository without leaking across tabs', async () => {
      addTestRepository(REPO_PATH_A, 'repo-a');
      addTestRepository(REPO_PATH_B, 'repo-b');

      const repoAFile1 = stagedFile('src/a1.ts', 'new');
      const repoAFile2 = stagedFile('src/a2.ts', 'modified');
      const repoBFile1 = stagedFile('src/b1.ts', 'modified');
      const repoBFile2 = stagedFile('src/b2.ts', 'deleted');

      const el = await renderCommitPanel([repoAFile1], REPO_PATH_A);
      expect(getSummaryInput(el).value).to.equal('add file src/a1.ts');

      // User writes a custom draft in repo A
      const summaryInput = getSummaryInput(el);
      summaryInput.value = 'Custom draft for repo A';
      summaryInput.dispatchEvent(new Event('input'));
      await el.updateComplete;

      // Switch to repo B with 1 staged file -> should auto-populate without inheriting repo A flags
      el.repositoryPath = REPO_PATH_B;
      el.stagedFiles = [repoBFile1];
      el.stagedCount = 1;
      await el.updateComplete;

      expect(getSummaryInput(el).value).to.equal('modified file src/b1.ts');
      expect(getDescriptionInput(el).value).to.equal('');

      // Repo B is still auto-generated, so adding a second file updates it
      await setStagedFiles(el, [repoBFile1, repoBFile2]);
      expect(getSummaryInput(el).value).to.equal('multiple changes');
      expect(getDescriptionInput(el).value).to.equal(
        'modified file src/b1.ts\ndeleted file src/b2.ts'
      );

      // Switch back to repo A with changed stagedFiles -> custom draft & hasCustomMessage are preserved
      el.repositoryPath = REPO_PATH_A;
      el.stagedFiles = [repoAFile1, repoAFile2];
      el.stagedCount = 2;
      await el.updateComplete;

      expect(getSummaryInput(el).value).to.equal('Custom draft for repo A');
      expect(getDescriptionInput(el).value).to.equal('');

      // Switch back to repo B with empty stagedFiles -> isAutoGeneratedMessage is preserved so it clears
      el.repositoryPath = REPO_PATH_B;
      el.stagedFiles = [];
      el.stagedCount = 0;
      await el.updateComplete;

      expect(getSummaryInput(el).value).to.equal('');
      expect(getDescriptionInput(el).value).to.equal('');
    });
  });

  // ── 8. Reset after successful commit ─────────────────────────────────────
  describe('after successful commit', () => {
    it('re-enables default commit message auto-fill for newly staged files', async () => {
      const firstFile = stagedFile('src/first.ts', 'new');
      const nextFile = stagedFile('src/next.ts', 'modified');
      const el = await renderCommitPanel([firstFile]);

      // Make a manual edit before committing
      const summaryInput = getSummaryInput(el);
      summaryInput.value = 'Manual commit before reset';
      summaryInput.dispatchEvent(new Event('input'));
      await el.updateComplete;

      const commitBtn = el.shadowRoot!.querySelector('.commit-btn') as HTMLButtonElement;
      expect(commitBtn.disabled).to.be.false;
      commitBtn.click();
      await flushAsync();
      await el.updateComplete;

      expect(invokeHistory.some((entry) => entry.command === 'create_commit')).to.be.true;
      expect(getSummaryInput(el).value).to.equal('');
      expect(getDescriptionInput(el).value).to.equal('');

      // Simulate post-commit status refresh clearing staged files, then staging a new file
      await setStagedFiles(el, []);
      await setStagedFiles(el, [nextFile]);

      expect(getSummaryInput(el).value).to.equal('modified file src/next.ts');
      expect(getDescriptionInput(el).value).to.equal('');
    });
  });
});
