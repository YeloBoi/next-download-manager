// Use Git Credential Manager without displaying or persisting its credential.
import { spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
const git = spawnSync('git', ['credential', 'fill'], {
  input: 'protocol=https\nhost=github.com\n\n', encoding: 'utf8',
  env: { ...process.env, GIT_TERMINAL_PROMPT: '0', GCM_INTERACTIVE: 'never' },
});
if (git.status !== 0) throw new Error('No saved GitHub credential is available. Sign in to GitHub with Git Credential Manager.');
const fields = Object.fromEntries(git.stdout.trim().split(/\r?\n/).map(line => {
  const at = line.indexOf('='); return [line.slice(0, at), line.slice(at + 1)];
}));
if (!fields.password) throw new Error('Git Credential Manager did not return a credential.');
const [method = 'GET', path = '/user', payload] = process.argv.slice(2);
const body = payload?.startsWith('@') ? await readFile(payload.slice(1), 'utf8') : payload;
if (!path.startsWith('/') || path.includes('://')) throw new Error('Use a GitHub API path.');
const response = await fetch(`https://api.github.com${path}`, {
  method, headers: { Accept: 'application/vnd.github+json', Authorization: `Bearer ${fields.password}`,
    'X-GitHub-Api-Version': '2022-11-28', 'Content-Type': 'application/json' }, body,
});
const content = await response.text();
if (response.ok && path.endsWith('/logs')) {
  const lines = content.split(/\r?\n/);
  const errors = lines.flatMap((line,index) => /error(?:\[|:)|warning:|panicked|FAILED|Exception|Cannot find/i.test(line)
    ? lines.slice(Math.max(0,index-2),Math.min(lines.length,index+9)) : []);
  console.log((errors.length ? [...new Set(errors)] : lines.slice(-50)).join('\n'));
  process.exit(0);
}
let result;
try { result = JSON.parse(content); } catch { result = {}; }
if (!response.ok) throw new Error(`GitHub ${response.status}: ${result.message || 'Request failed'}`);
if (path === '/user') console.log(JSON.stringify({ login: result.login, id: result.id }));
else if (result.full_name) console.log(JSON.stringify({ name: result.full_name, url: result.html_url, private: result.private }));
else if (result.workflow_runs) console.log(JSON.stringify(result.workflow_runs.map(run => ({ id: run.id, name: run.name, status: run.status, conclusion: run.conclusion, url: run.html_url, sha: run.head_sha }))));
else if (result.jobs) console.log(JSON.stringify(result.jobs.map(job => ({ id: job.id, status: job.status, conclusion: job.conclusion, steps: job.steps.map(step => ({ name: step.name, status: step.status, conclusion: step.conclusion })) }))));
else console.log(JSON.stringify(result));
