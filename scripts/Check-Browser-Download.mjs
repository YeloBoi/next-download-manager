// Real release executables, HTTP bytes and Windows job lifetime; hosted CI only.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';

assert.equal(process.env.GITHUB_ACTIONS, 'true');
assert.equal(process.platform, 'win32');
const payload = Buffer.alloc(128 * 1024);
for (let i = 0; i < payload.length; i++) payload[i] = i % 251;
const hash = createHash('sha256').update(payload).digest('hex');
const received = new Set();
const server = createServer((request, response) => {
  received.add(request.url);
  const range = /^bytes=(\d+)-(\d*)$/.exec(request.headers.range ?? '');
  const start = range ? Number(range[1]) : 0;
  const end = range?.[2] ? Number(range[2]) : payload.length - 1;
  assert.ok(start <= end && end < payload.length);
  response.writeHead(range ? 206 : 200, {
    'Content-Type': 'application/octet-stream',
    'Content-Length': end - start + 1,
    'Accept-Ranges': 'bytes',
    ETag: '"ndm-ci-download-v1"',
    ...(range ? { 'Content-Range': `bytes ${start}-${end}/${payload.length}` } : {}),
  });
  response.flushHeaders();
  // The download must continue after the browser kills the one-shot native host.
  if (request.method === 'HEAD') response.end();
  else if (end === start) response.end(payload.subarray(start, end + 1));
  else setTimeout(() => response.end(payload.subarray(start, end + 1)), 3000);
});
try {
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const code = await new Promise((resolve, reject) => {
    const child = spawn('pwsh', ['-NoProfile', '-File', 'scripts/Check-Native-Download.ps1',
      '-UrlBase', `http://127.0.0.1:${server.address().port}`, '-ExpectedHash', hash], { stdio: 'inherit' });
    child.on('error', reject);
    child.on('exit', resolve);
  });
  assert.equal(code, 0, 'Real Windows desktop/native download acceptance');
  assert.ok([...received].some(path => path.includes('cold-')), 'Cold launch fetched HTTP bytes');
  assert.ok([...received].some(path => path.includes('warm-')), 'Existing desktop fetched HTTP bytes');
  console.log('Native download checks passed: cold launch, browser job teardown, visible window, warm restore, one desktop instance and SHA-256 verified files.');
} finally {
  server.closeAllConnections();
  await new Promise(resolve => server.close(resolve));
}
