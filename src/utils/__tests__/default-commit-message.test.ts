import { expect } from '@open-wc/testing';
import {
  buildDefaultCommitMessage,
  formatStagedFileLine,
} from '../default-commit-message.ts';

describe('default-commit-message', () => {
  describe('formatStagedFileLine', () => {
    it('formats new and untracked files with "add file"', () => {
      expect(formatStagedFileLine({ path: 'README.md', status: 'new' })).to.equal(
        'add file README.md'
      );
      expect(formatStagedFileLine({ path: 'notes.txt', status: 'untracked' })).to.equal(
        'add file notes.txt'
      );
    });

    it('formats modified files with "modified file"', () => {
      expect(formatStagedFileLine({ path: 'index.ts', status: 'modified' })).to.equal(
        'modified file index.ts'
      );
    });

    it('formats deleted files with "deleted file"', () => {
      expect(formatStagedFileLine({ path: 'old.ts', status: 'deleted' })).to.equal(
        'deleted file old.ts'
      );
    });

    it('formats renamed files with "renamed file"', () => {
      expect(formatStagedFileLine({ path: 'new-name.ts', status: 'renamed' })).to.equal(
        'renamed file new-name.ts'
      );
    });

    it('formats copied files with "copied file"', () => {
      expect(formatStagedFileLine({ path: 'copy.ts', status: 'copied' })).to.equal(
        'copied file copy.ts'
      );
    });

    it('formats typechange files with "changed file"', () => {
      expect(formatStagedFileLine({ path: 'link.sh', status: 'typechange' })).to.equal(
        'changed file link.sh'
      );
    });

    it('falls back to "update file" for conflicted and ignored statuses', () => {
      expect(formatStagedFileLine({ path: 'conflict.ts', status: 'conflicted' })).to.equal(
        'update file conflict.ts'
      );
      expect(formatStagedFileLine({ path: 'ignored.log', status: 'ignored' })).to.equal(
        'update file ignored.log'
      );
    });

    it('preserves subdirectory paths', () => {
      expect(
        formatStagedFileLine({
          path: 'src/components/panels/gn-commit-panel.ts',
          status: 'modified',
        })
      ).to.equal('modified file src/components/panels/gn-commit-panel.ts');
    });
  });

  describe('buildDefaultCommitMessage', () => {
    it('returns empty summary and description when there are no staged files', () => {
      expect(buildDefaultCommitMessage([])).to.deep.equal({
        summary: '',
        description: '',
      });
    });

    it('sets summary and empty description for a single staged file', () => {
      expect(
        buildDefaultCommitMessage([{ path: 'src/utils/helper.ts', status: 'new' }])
      ).to.deep.equal({
        summary: 'add file src/utils/helper.ts',
        description: '',
      });

      expect(
        buildDefaultCommitMessage([{ path: 'src/app.ts', status: 'modified' }])
      ).to.deep.equal({
        summary: 'modified file src/app.ts',
        description: '',
      });

      expect(
        buildDefaultCommitMessage([{ path: 'legacy/old.js', status: 'deleted' }])
      ).to.deep.equal({
        summary: 'deleted file legacy/old.js',
        description: '',
      });

      expect(
        buildDefaultCommitMessage([{ path: 'src/renamed.ts', status: 'renamed' }])
      ).to.deep.equal({
        summary: 'renamed file src/renamed.ts',
        description: '',
      });

      expect(
        buildDefaultCommitMessage([{ path: 'src/copied.ts', status: 'copied' }])
      ).to.deep.equal({
        summary: 'copied file src/copied.ts',
        description: '',
      });

      expect(
        buildDefaultCommitMessage([{ path: 'scripts/run', status: 'typechange' }])
      ).to.deep.equal({
        summary: 'changed file scripts/run',
        description: '',
      });

      expect(
        buildDefaultCommitMessage([{ path: 'src/conflict.ts', status: 'conflicted' }])
      ).to.deep.equal({
        summary: 'update file src/conflict.ts',
        description: '',
      });
    });

    it('uses "multiple changes" summary and line-by-line body in input order for multiple files', () => {
      const staged = [
        { path: 'src/utils/default-commit-message.ts', status: 'new' as const },
        { path: 'src/components/panels/gn-commit-panel.ts', status: 'modified' as const },
        { path: 'docs/obsolete.md', status: 'deleted' as const },
        { path: 'src/renamed-file.ts', status: 'renamed' as const },
      ];

      expect(buildDefaultCommitMessage(staged)).to.deep.equal({
        summary: 'multiple changes',
        description: [
          'add file src/utils/default-commit-message.ts',
          'modified file src/components/panels/gn-commit-panel.ts',
          'deleted file docs/obsolete.md',
          'renamed file src/renamed-file.ts',
        ].join('\n'),
      });
    });
  });
});
