import { createServer } from 'node:http';
import { readFile, realpath, stat } from 'node:fs/promises';
import path from 'node:path';
import { build } from './build.mjs';
import { distRoot, within } from './shared.mjs';

await build();
const canonicalRoot = await realpath(distRoot);
const types = {
  '.html': 'text/html; charset=utf-8', '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8', '.json': 'application/json; charset=utf-8',
  '.xml': 'application/xml; charset=utf-8',
  '.svg': 'image/svg+xml', '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg',
  '.webp': 'image/webp', '.ico': 'image/x-icon', '.woff': 'font/woff', '.woff2': 'font/woff2',
  '.md': 'text/plain; charset=utf-8', '.txt': 'text/plain; charset=utf-8',
  '.sha256': 'text/plain; charset=utf-8', '.crexe': 'application/octet-stream',
  '.zip': 'application/zip', '.gz': 'application/gzip',
};

const server = createServer(async (request, response) => {
  response.setHeader('X-Content-Type-Options', 'nosniff');
  response.setHeader('Cache-Control', 'no-store');
  if (request.method !== 'GET' && request.method !== 'HEAD') {
    response.writeHead(405, { Allow: 'GET, HEAD' });
    response.end('Method not allowed');
    return;
  }
  try {
    const pathname = decodeURIComponent(new URL(request.url, 'http://127.0.0.1').pathname);
    if (pathname.includes('\0') || pathname.includes('\\')) throw new Error('Invalid path');
    let filename = path.resolve(distRoot, `.${pathname}`);
    if (!within(distRoot, filename)) throw new Error('Invalid path');
    if ((await stat(filename)).isDirectory()) filename = path.join(filename, 'index.html');
    const canonicalFile = await realpath(filename);
    if (!within(canonicalRoot, canonicalFile) || !(await stat(canonicalFile)).isFile()) {
      throw new Error('Invalid path');
    }
    const body = await readFile(canonicalFile);
    const extension = path.extname(canonicalFile).toLowerCase();
    const headers = { 'Content-Type': types[extension] ?? 'application/octet-stream', 'Content-Length': body.length };
    if (extension === '.crexe' || extension === '.zip' || extension === '.gz') {
      headers['Content-Disposition'] = `attachment; filename="${path.basename(canonicalFile).replaceAll('"', '')}"`;
    }
    response.writeHead(200, headers);
    response.end(request.method === 'HEAD' ? undefined : body);
  } catch {
    response.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' });
    response.end(request.method === 'HEAD' ? undefined : 'Not found');
  }
});

server.on('error', (error) => {
  console.error(`Preview server: ${error.message}`);
  process.exitCode = 1;
});
server.listen(4173, '127.0.0.1', () => console.log('Local: http://127.0.0.1:4173'));
for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => server.close(() => process.exit(0)));
}
