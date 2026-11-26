import { createHash } from 'node:crypto';
import { copyFile, cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { buildPublications } from './publications.mjs';
import { canonicalPath } from './locales.mjs';
import {
  collectDocumentation, distRoot, examples, git, readJson, repoRoot, siteRoot,
  staticFiles, validateDownloads, within,
} from './shared.mjs';

export async function build() {
  validateDownloads(await readJson(path.join(siteRoot, 'downloads.json')));
  const documentation = await collectDocumentation();
  // Publish the output-material license at its original path, including when no
  // maintained Markdown has linked to it yet. Source archives still use HEAD.
  if (!documentation.files.some(([, destination]) => destination === 'LICENSE-GENERATED')) {
    documentation.files.push(['LICENSE-GENERATED', 'LICENSE-GENERATED']);
  }
  // Preserve the supplied draft schema at the canonical $id declared in it.
  documentation.files.push(['docs/formatos/originais/crexb-manifest-v1.schema.json', 'schemas/crexb-manifest-v1.schema.json']);
  // Validate source inputs before replacing a previous build.
  await Promise.all(staticFiles.map((file) => readFile(path.join(siteRoot, file))));
  await readFile(path.join(siteRoot, 'download-counts-worker.js'));
  await readFile(path.join(repoRoot, 'LICENSE-GENERATED'));
  const revision = git(['rev-parse', '--verify', 'HEAD']).trim();
  const shortRevision = revision.slice(0, 12);
  const archiveName = `crexe-source-${shortRevision}.zip`;
  // Git reads committed HEAD only. Working-tree edits, ignored secrets, caches,
  // and untracked files never enter the source download. The site is separate.
  // Select tree entries because git archive does not support exclude pathspecs.
  const rootEntries = git(['ls-tree', '-z', '--name-only', revision]).split('\0').filter(Boolean);
  const archivePaths = rootEntries.flatMap((entry) => {
    if (entry !== 'src' && entry !== 'docs') return [entry];
    const excluded = entry === 'src' ? 'site' : 'archive';
    return git(['ls-tree', '-z', '--name-only', `${revision}:${entry}`])
      .split('\0').filter((name) => name && name !== excluded).map((name) => `${entry}/${name}`);
  });

  if (!within(siteRoot, distRoot) || path.basename(distRoot) !== 'dist' || distRoot === siteRoot) {
    throw new Error('Refusing to replace a build directory outside src/site/dist.');
  }
  await rm(distRoot, { recursive: true, force: true });
  await mkdir(path.join(distRoot, 'downloads'), { recursive: true });
  await Promise.all(staticFiles.map(async (file) => {
    await mkdir(path.dirname(path.join(distRoot, file)), { recursive: true });
    await copyFile(path.join(siteRoot, file), path.join(distRoot, file));
  }));
  // Pages advanced mode; the routes file keeps ordinary pages/downloads static.
  await copyFile(path.join(siteRoot, 'download-counts-worker.js'), path.join(distRoot, '_worker.js'));
  for (const page of ['index.html', 'en/index.html']) {
    const landing = await readFile(path.join(distRoot, page), 'utf8');
    await writeFile(path.join(distRoot, page), landing.replaceAll('downloads/crexe-source.zip', `downloads/${archiveName}`), 'utf8');
  }
  await cp(path.join(siteRoot, 'assets'), path.join(distRoot, 'assets'), { recursive: true });
  await Promise.all([...documentation.files, ...examples].map(async ([source, destination]) => {
    const output = path.join(distRoot, destination);
    await mkdir(path.dirname(output), { recursive: true });
    await copyFile(path.join(repoRoot, source), output);
  }));
  const publications = await buildPublications();
  // Cached stylesheet URLs must change when their contents change, including
  // for returning visitors whose browser still considers the old CSS fresh.
  const styleVersions = new Map(await Promise.all(['styles', 'publications'].map(async (name) =>
    [name, createHash('sha256').update(await readFile(path.join(distRoot, `${name}.css`))).digest('hex').slice(0, 12)])));
  for (const page of ['index.html', 'en/index.html', '404.html', ...publications.pages]) {
    const filename = path.join(distRoot, page);
    const html = await readFile(filename, 'utf8');
    await writeFile(filename, html.replace(/href="((?:\.\.\/)*(styles|publications)\.css)"/g,
      (_, href, name) => `href="${href}?v=${styleVersions.get(name)}"`), 'utf8');
  }
  const publicationUrls = ['index.html', 'en/index.html', ...publications.pages].map((page) => {
    const entry = publications.catalog.find((item) => item.url === page);
    const url = canonicalPath(page);
    return `  <url><loc>https://crexe.org/${url}</loc>${entry ? `<lastmod>${entry.date}</lastmod>` : ''}</url>`;
  });
  await writeFile(path.join(distRoot, 'sitemap.xml'), `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${publicationUrls.join('\n')}\n</urlset>\n`, 'utf8');
  const archivePath = path.join(distRoot, 'downloads', archiveName);
  git(['archive', '--format=zip', `--output=${archivePath}`, `--prefix=crexe-${shortRevision}/`, revision, '--', ...archivePaths]);
  const checksum = createHash('sha256').update(await readFile(archivePath)).digest('hex');
  await writeFile(`${archivePath}.sha256`, `${checksum}  ${archiveName}\n`, 'utf8');
  await writeFile(path.join(distRoot, 'source.json'), `${JSON.stringify({
    revision,
    shortRevision,
    url: `downloads/${archiveName}`,
    checksumUrl: `downloads/${archiveName}.sha256`,
  }, null, 2)}\n`, 'utf8');
  console.log(`Built ${distRoot}`);
  console.log(`Source download contains committed HEAD ${shortRevision}; current uncommitted edits are excluded.`);
  return { revision, shortRevision, documentation, publications };
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  await build();
}
