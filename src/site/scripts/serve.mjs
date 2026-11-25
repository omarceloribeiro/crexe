import path from 'node:path';
import { build } from './build.mjs';
import { distRoot } from './shared.mjs';
import { createStaticServer } from './hosting.mjs';

const args = process.argv.slice(2);
if (args.length && (args.length !== 2 || args[0] !== '--directory')) throw new Error('Usage: node scripts/serve.mjs [--directory ABSOLUTE_SITE_DIRECTORY]');
if (!args.length) await build();
const root = args.length ? path.resolve(args[1]) : distRoot;
const server = await createStaticServer(root);
server.on('error', (error) => { console.error(`Preview server: ${error.message}`); process.exitCode = 1; });
server.listen(4173, '127.0.0.1', () => console.log(`Directory: ${root}\nLocal: http://127.0.0.1:4173`));
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => server.close(() => process.exit(0)));
