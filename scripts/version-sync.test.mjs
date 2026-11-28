import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

const read = (path) => readFileSync(join(import.meta.dirname, '..', path), 'utf8');
const { version } = JSON.parse(read('package.json'));

test('native release versions match package.json', () => {
  const manifest = read('src-tauri/Cargo.toml').split(/(?=^\[)/m)
    .find((section) => section.startsWith('[package]'));
  const locked = read('src-tauri/Cargo.lock').split('[[package]]')
    .find((section) => /^name = "gitduda"\r?$/m.test(section) && !/^source = /m.test(section));
  for (const [file, section] of [['Cargo.toml', manifest], ['Cargo.lock', locked]]) {
    assert.equal(section?.match(/^version\s*=\s*"([^"]+)"/m)?.[1], version,
      file + ' must match package.json. Run npm run sync-version.');
  }
  assert.equal(JSON.parse(read('src-tauri/tauri.conf.json')).version, version,
    'tauri.conf.json must match package.json. Run npm run sync-version.');
});
