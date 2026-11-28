import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { createInterface } from 'node:readline/promises';
import prompts from 'prompts';
import semver from 'semver';

const cwd = join(import.meta.dirname, '..');
const files = ['package.json', 'package-lock.json', 'src-tauri/Cargo.toml',
  'src-tauri/Cargo.lock', 'src-tauri/tauri.conf.json'];
const run = (command, args) => execFileSync(command, args, { cwd, stdio: 'inherit' });
const npm = (...args) => run(process.execPath, [process.env.npm_execpath, ...args]);

async function ask(question) {
  const prompt = createInterface({ input: process.stdin, output: process.stdout });
  try {
    return (await prompt.question(question)).trim().toLowerCase();
  } finally {
    prompt.close();
  }
}

const increments = ['patch', 'minor', 'major'];

async function chooseIncrement() {
  if (!process.stdin.isTTY || !process.stdout.isTTY) {
    return ask('Incremento da versão (patch/minor/major)? ');
  }
  const { version } = JSON.parse(readFileSync(join(cwd, 'package.json'), 'utf8'));
  const { bump } = await prompts({
    type: 'select',
    name: 'bump',
    message: `Qual será a próxima versão após ${version}?`,
    hint: 'Use ↑ e ↓ para escolher e Enter para confirmar.',
    choices: increments.map((value) => ({ value, title: `${value} → ${semver.inc(version, value)}` })),
  });
  return bump;
}

async function prepare() {
  const bump = process.argv[2] ?? await chooseIncrement();
  if (bump === undefined) return;
  if (!increments.includes(bump) || process.argv.length > 3) {
    throw new Error('Use npm run release:prepare -- patch, minor ou major.');
  }

  // A path-limited commit includes all edits to each named file.
  const pending = execFileSync('git', ['status', '--porcelain', '--', ...files], { cwd, encoding: 'utf8' });
  if (pending.trim()) {
    throw new Error('Revise as alterações pendentes nos arquivos de versão antes de continuar.\n' + pending);
  }

  npm('run', 'test:contract');
  npm('version', bump, '--no-git-tag-version');
  run(process.execPath, ['--test', 'scripts/version-sync.test.mjs']);
  const { version } = JSON.parse(readFileSync(join(cwd, 'package.json'), 'utf8'));
  run('git', ['--no-pager', 'diff', 'HEAD', '--', ...files]);
  const answer = await ask(`Criar o commit local da versão ${version}? [s/N] `);
  if (['s', 'sim'].includes(answer)) {
    run('git', ['commit', '--only', '-m', `Preparar a versão ${version}`,
      '-m', `Sincroniza os metadados de versão para gerar os pacotes da release ${version}.`,
      '--', ...files]);
  } else {
    console.log(`Versão ${version} preparada para revisão.`);
  }
}

prepare().catch((error) => {
  console.error(error.message);
  process.exitCode = 1;
});
