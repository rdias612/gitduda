import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const root = join(import.meta.dirname, '..');
const read = (path) => readFileSync(join(root, path), 'utf8');
const { version } = JSON.parse(read('package.json'));
if (typeof version !== 'string' || !/^[0-9A-Za-z.+-]+$/.test(version)) {
  throw new Error('Expected a release version in package.json');
}

function updateCargo(text, lockfile) {
  // Keep comments, dependency versions and the original line endings intact.
  const sections = text.split(/(?=^\[)/m);
  const matches = sections.filter((section) => lockfile
    ? /^\[\[package\]\]\r?$/m.test(section) && /^name = "gitduda"\r?$/m.test(section)
      && !/^source = /m.test(section)
    : /^\[package\]\r?$/m.test(section));
  const versionLine = /^(version\s*=\s*)"[^"]+"/m;
  if (matches.length !== 1 || !versionLine.test(matches[0])) {
    throw new Error('Expected one local Cargo package with a version');
  }
  return sections.map((section) => section === matches[0]
    ? section.replace(versionLine, '$1' + JSON.stringify(version))
    : section).join('');
}

function updateTauri(text) {
  const config = JSON.parse(text);
  if (config.version === version) return text;
  const updated = text.replace(/^([ \t]*"version"\s*:\s*)"[^"]+"/m, '$1' + JSON.stringify(version));
  if (JSON.parse(updated).version !== version) {
    throw new Error('Expected a top-level version in tauri.conf.json');
  }
  return updated;
}

const targets = [
  ['src-tauri/Cargo.toml', (text) => updateCargo(text, false)],
  ['src-tauri/Cargo.lock', (text) => updateCargo(text, true)],
  ['src-tauri/tauri.conf.json', updateTauri],
];
const changes = [];
for (const [path, update] of targets) {
  const original = read(path);
  const updated = update(original);
  if (updated !== original) changes.push([path, updated]);
}
for (const [path, updated] of changes) writeFileSync(join(root, path), updated);

// npm stages its own manifests. Include the native files in the same release commit.
const gitTagVersion = process.env.npm_config_git_tag_version ?? 'true';
if (changes.length && process.argv.includes('--stage') && gitTagVersion === 'true'
    && existsSync(join(root, '.git'))) {
  execFileSync('git', ['add', '--', ...changes.map(([path]) => path)], { cwd: root });
}
