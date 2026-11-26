import { mkdir, readFile, realpath, writeFile } from 'node:fs/promises';
import path from 'node:path';
import MarkdownIt from 'markdown-it';
import { distRoot, readJson, repoRoot, siteRoot, within } from './shared.mjs';
import { canonicalPath, getLocale, publicationPath, supportedLocales } from './locales.mjs';

const { categoryLabels } = getLocale();

export function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (character) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  })[character]);
}

function repositoryPath(value) {
  if (typeof value !== 'string' || !value || /[\\\u0000-\u001f]/.test(value)
    || path.posix.isAbsolute(value) || /^[a-z][a-z\d+.-]*:/i.test(value)) {
    throw new Error(`Invalid repository path: ${value}`);
  }
  const normalized = path.posix.normalize(value);
  if (normalized === '..' || normalized.startsWith('../')) throw new Error(`Path escapes repository: ${value}`);
  return normalized;
}

function urlPath(value) {
  return value.split('/').map((segment) => encodeURIComponent(segment)).join('/');
}

export function validateCatalog(catalog) {
  if (!Array.isArray(catalog) || !catalog.length) throw new Error('Publication catalog must be a nonempty array.');
  const slugs = new Set();
  const sources = new Set();
  return catalog.map((entry) => {
    for (const field of ['slug', 'title', 'description', 'category', 'status', 'date', 'notice', 'source']) {
      if (typeof entry?.[field] !== 'string' || !entry[field].trim()) throw new Error(`Publication requires ${field}.`);
    }
    if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(entry.slug) || entry.slug === 'index' || slugs.has(entry.slug)) {
      throw new Error(`Invalid or duplicate publication slug: ${entry.slug}`);
    }
    if (!Object.hasOwn(categoryLabels, entry.category)) throw new Error(`Unknown publication category: ${entry.category}`);
    const date = new Date(`${entry.date}T00:00:00Z`);
    if (!/^\d{4}-\d{2}-\d{2}$/.test(entry.date) || Number.isNaN(date.valueOf()) || date.toISOString().slice(0, 10) !== entry.date) {
      throw new Error(`Invalid publication date: ${entry.date}`);
    }
    const source = repositoryPath(entry.source);
    if (!source.endsWith('.md') || sources.has(source)) throw new Error(`Invalid or duplicate publication source: ${source}`);
    if (entry.language !== undefined) getLocale(entry.language);
    if (entry.originalSource !== undefined) repositoryPath(entry.originalSource);
    if (entry.originalLanguage !== undefined) {
      if (!entry.originalSource) throw new Error('An original language requires an original source.');
      getLocale(entry.originalLanguage);
    }
    slugs.add(entry.slug);
    sources.add(source);
    return { ...entry, source };
  });
}

function headingSlug(title) {
  return title.normalize('NFKD').replace(/\p{M}/gu, '').toLowerCase()
    .replace(/[^\p{L}\p{N}\s_-]/gu, '').trim().replace(/[\s_]+/g, '-').replace(/-+/g, '-') || 'secao';
}

function plainInline(token) {
  if (!token) return '';
  if (token.children) return token.children.map(plainInline).join('');
  if (token.type === 'softbreak' || token.type === 'hardbreak') return ' ';
  return token.type === 'text' || token.type === 'code_inline' ? token.content : '';
}

function rewriteLocalUrl(value, entry, catalog, localLinks, kind) {
  if (value.startsWith('#')) return value;
  if (/^(?:https?:|mailto:|\/\/)/i.test(value)) return value;
  if (/^[a-z][a-z\d+.-]*:/i.test(value)) throw new Error(`Unsupported URL scheme in ${entry.source}.`);
  const match = value.match(/^([^?#]*)([?#].*)?$/);
  const relative = decodeURIComponent(match[1]);
  const suffix = match[2] ?? '';
  if (relative.startsWith('/') || /[\\\u0000-\u001f]/.test(relative)) {
    throw new Error(`Invalid local URL in ${entry.source}: ${value}`);
  }
  const target = repositoryPath(relative ? path.posix.join(path.posix.dirname(entry.source), relative) : entry.source);
  const originalDownload = target === entry.originalSource && target !== entry.source;
  const publication = !originalDownload && catalog.find((item) => repositoryPath(item.source) === target || item.sourceAliases?.includes(target));
  // README language links must work both as relative GitHub links and as links
  // between reader editions. Ordinary cross-language references stay localized.
  const readmeLanguages = new Map([['README.md', 'en'], ['README-pt-BR.md', 'pt-BR']]);
  const targetLanguage = readmeLanguages.get(target);
  const readmeTranslation = publication && readmeLanguages.has(entry.source)
    && targetLanguage && targetLanguage !== entry.language;
  const destination = readmeTranslation
    ? path.posix.relative(path.posix.dirname(publicationPath(entry.language, entry.slug)), publicationPath(targetLanguage, publication.slug))
    : publication ? `${publication.slug}.html` : `${getLocale(entry.language).assetPrefix}${urlPath(target)}`;
  const url = `${destination}${suffix}`;
  localLinks.push({ source: entry.source, target, url, kind });
  return url;
}

/** Render maintained Markdown without trusting HTML, URL schemes or local paths. */
export function renderPublication(markdown, entry, catalog) {
  const source = repositoryPath(entry.source);
  const current = { ...entry, source };
  const locale = getLocale(entry.language);
  const md = new MarkdownIt({ html: false, linkify: false, typographer: false });
  const defaultValidate = md.validateLink.bind(md);
  md.validateLink = (value) => defaultValidate(value)
    && !/[\u0000-\u001f]/.test(value)
    && (!/^[a-z][a-z\d+.-]*:/i.test(value) || /^(?:https?:|mailto:)/i.test(value));
  const environment = {};
  const tokens = md.parse(markdown, environment);
  const firstTitle = tokens.findIndex((token) => token.type === 'heading_open' && token.tag === 'h1');
  if (firstTitle >= 0 && tokens[firstTitle + 2]?.type === 'heading_close') {
    tokens.splice(firstTitle, 3);
    // Authored publications repeat their status/date in a conventional opening
    // quote. Page metadata supplies those; substantive quotations remain intact.
    const quote = tokens.slice(firstTitle, firstTitle + 5);
    const quoteText = plainInline(quote[2]);
    if (quote.map((token) => token.type).join(',') === 'blockquote_open,paragraph_open,inline,paragraph_close,blockquote_close'
      && /^(?:Artigo|Ensaio|Manifesto|Guia|White\s?paper|Article|Essay|Guide|Technical guide)[^•·\n]{0,40}[•·]/iu.test(quoteText)
      && [dateLabel(entry.date), dateLabel(entry.date, 'en'), entry.date].some((date) => quoteText.includes(date))) tokens.splice(firstTitle, 5);
  }

  const toc = [];
  const usedIds = new Set(['artigo', 'conteudo', 'article-title', 'related-title']);
  const localLinks = [];
  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (token.type !== 'heading_open') continue;
    const title = plainInline(tokens[index + 1]).trim();
    const base = headingSlug(title);
    let id = base;
    let number = 2;
    while (usedIds.has(id)) id = `${base}-${number++}`;
    usedIds.add(id);
    token.attrSet('id', id);
    if (token.tag === 'h2' || token.tag === 'h3') toc.push({ id, title, level: Number(token.tag.slice(1)) });
  }
  const visit = (token) => {
    for (const [attribute, kind] of [['href', 'link'], ['src', 'image']]) {
      const value = token.attrGet(attribute);
      if (value !== null) token.attrSet(attribute, rewriteLocalUrl(value, current, catalog, localLinks, kind));
    }
    if (token.type === 'image') {
      token.attrSet('loading', 'lazy');
      token.attrSet('decoding', 'async');
    }
    token.children?.forEach(visit);
  };
  tokens.forEach(visit);
  md.renderer.rules.table_open = (items, index, options, env, renderer) =>
    `<div class="table-scroll" role="region" aria-label="${locale.table}" tabindex="0">${renderer.renderToken(items, index, options)}`;
  md.renderer.rules.table_close = (items, index, options, env, renderer) =>
    `${renderer.renderToken(items, index, options)}</div>\n`;
  const text = tokens.map((token) => token.type === 'inline' ? plainInline(token)
    : ['fence', 'code_block'].includes(token.type) ? token.content : '').join(' ');
  const wordCount = (text.match(/[\p{L}\p{N}]+(?:['’\-][\p{L}\p{N}]+)*/gu) ?? []).length;
  return { html: md.renderer.render(tokens, md.options, environment), toc, wordCount, readTime: Math.max(1, Math.ceil(wordCount / 200)), localLinks };
}

function dateLabel(value, language = 'pt-BR') {
  return new Intl.DateTimeFormat(language, { timeZone: 'UTC', day: '2-digit', month: language === 'en' ? 'short' : '2-digit', year: 'numeric' })
    .format(new Date(`${value}T00:00:00Z`));
}

function shell({ title, description, language, slug = 'index', bodyClass, content }) {
  const locale = getLocale(language);
  const page = publicationPath(language, slug);
  const alternateLinks = supportedLocales.map((other) => `<link rel="alternate" hreflang="${other}" href="https://crexe.org/${canonicalPath(publicationPath(other, slug))}">`).join('\n  ');
  const languageLinks = supportedLocales.map((other) => `<a lang="${other}" hreflang="${other}" aria-label="${other === 'en' ? 'English' : 'Português'}" href="${path.posix.relative(path.posix.dirname(page), publicationPath(other, slug))}"${other === language ? ' aria-current="page"' : ''}>${other === 'en' ? 'EN' : 'PT'}</a>`).join('');
  const canonical = `https://crexe.org/${canonicalPath(page)}`;
  return `<!doctype html>
<html lang="${language}">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>${escapeHtml(title)} · CREXE</title>
  <meta name="description" content="${escapeHtml(description)}">
  <link rel="canonical" href="${escapeHtml(canonical)}">
  ${alternateLinks}
  <link rel="alternate" hreflang="x-default" href="https://crexe.org/${canonicalPath(publicationPath('pt-BR', slug))}">
  <meta property="og:type" content="${bodyClass === 'reading-page' ? 'article' : 'website'}">
  <meta property="og:title" content="${escapeHtml(title)}">
  <meta property="og:description" content="${escapeHtml(description)}">
  <meta property="og:url" content="${escapeHtml(canonical)}">
  <link rel="icon" type="image/svg+xml" href="${locale.assetPrefix}assets/mark.svg">
  <link rel="stylesheet" href="${locale.assetPrefix}styles.css">
  <link rel="stylesheet" href="${locale.assetPrefix}publications.css">
  <script type="module" src="${locale.assetPrefix}library.js"></script>
</head>
<body class="${bodyClass}">
  <a class="skip-link" href="#conteudo">${locale.skip}</a>
  <header class="site-header"><div class="container header-inner">
    <a class="brand" href="../index.html" aria-label="${locale.home}"><img src="${locale.assetPrefix}assets/mark.svg" alt="" width="32" height="32"><span>crexe<span class="brand-period">.</span></span></a>
    <nav class="library-nav" aria-label="${locale.navigation}"><a href="index.html"${bodyClass === 'library-page' ? ' aria-current="page"' : ''}>${locale.libraryTitle}</a><a href="../index.html#downloads">Downloads</a></nav>
    <div class="language-switch" data-language-switch role="group" aria-label="${locale.languages}">${languageLinks}</div>
  </div></header>
  ${content}
  <footer class="library-footer container"><p>CREXE · Creative Executable</p><nav aria-label="${locale.projectLinks}"><a href="../index.html">${locale.project}</a><a href="${locale.assetPrefix}LICENSE.txt">MIT</a><a href="security.html">${locale.security}</a></nav></footer>
</body>
</html>
`;
}

function publicationCard(entry) {
  const locale = getLocale(entry.language);
  return `<article class="publication-card">
    <div class="publication-card-meta"><span>${escapeHtml(locale.categoryLabels[entry.category])}</span><span>${escapeHtml(entry.status)}</span></div>
    <h3><a href="${entry.slug}.html">${escapeHtml(entry.title)}</a></h3>
    <p>${escapeHtml(entry.description)}</p>
    <div class="publication-card-footer"><span>${entry.readTime} ${locale.minutes}</span><a href="${entry.slug}.html" aria-label="${escapeHtml(`${locale.read} ${entry.title}`)}">${locale.read} ${escapeHtml(locale.categoryLabels[entry.category].toLowerCase())}</a></div>
  </article>`;
}

function articlePage(entry, rendered, catalog) {
  const locale = getLocale(entry.language);
  const originalLanguage = entry.originalLanguage ?? 'en';
  const originalLabel = locale.originals[originalLanguage];
  const related = [...catalog.filter((other) => other.slug !== entry.slug && other.category === entry.category),
    ...catalog.filter((other) => other.slug !== entry.slug && other.category !== entry.category)].slice(0, 3);
  const toc = rendered.toc.length ? `<nav class="article-toc" aria-label="${locale.toc}"><details open><summary>${locale.inThisText}</summary><ol>${rendered.toc.map((heading) =>
    `<li${heading.level === 3 ? ' class="toc-subsection"' : ''}><a href="#${escapeHtml(heading.id)}">${escapeHtml(heading.title)}</a></li>`).join('')}</ol></details></nav>` : '';
  return shell({
    title: entry.title, description: entry.description, language: entry.language, slug: entry.slug, bodyClass: 'reading-page',
    content: `<main class="container" id="conteudo">
    <div class="article-breadcrumb"><a href="index.html">${locale.libraryTitle}</a><span aria-hidden="true">/</span><span>${escapeHtml(locale.categoryLabels[entry.category])}</span></div>
    <header class="article-heading"><p class="eyebrow">${escapeHtml(locale.categoryLabels[entry.category])}</p><h1 id="article-title">${escapeHtml(entry.title)}</h1><p class="article-description">${escapeHtml(entry.description)}</p><div class="article-meta"><span>${escapeHtml(entry.status)}</span><time datetime="${entry.date}">${dateLabel(entry.date, entry.language)}</time><span>${entry.readTime} ${locale.minutes}</span></div></header>
    <aside class="article-notice" aria-label="${locale.scope}">${escapeHtml(entry.notice)}</aside>
    <div class="reading-layout">${toc}<article class="article-prose" id="artigo" lang="${escapeHtml(entry.language ?? 'pt-BR')}" aria-labelledby="article-title">${rendered.html}</article></div>
    <div class="article-actions"><a href="${locale.assetPrefix}${urlPath(entry.source)}" download>${entry.source === entry.originalSource ? originalLabel : locale.download}</a>${entry.originalSource && entry.originalSource !== entry.source ? `<a href="${locale.assetPrefix}${urlPath(entry.originalSource)}" download hreflang="${originalLanguage}" data-original-download>${originalLabel}</a>` : ''}<button type="button" data-print hidden>${locale.print}</button><a href="index.html">${locale.back}</a></div>
    <section class="related-reading" aria-labelledby="related-title"><h2 id="related-title">${locale.related}</h2><div class="publication-grid">${related.map(publicationCard).join('\n')}</div></section>
  </main>`,
  });
}

function libraryPage(catalog, language) {
  const locale = getLocale(language);
  const visibleGroups = locale.groups.filter((group) => catalog.some((entry) => group.categories.includes(entry.category)));
  const featured = catalog.find((entry) => entry.category === 'whitepaper') ?? catalog[0];
  return shell({
    title: locale.libraryTitle, description: locale.libraryDescription, language, bodyClass: 'library-page',
    content: `<main class="container" id="conteudo">
    <header class="library-intro"><p class="eyebrow">${locale.introEyebrow}</p><h1>${locale.introTitle}</h1><p>${locale.intro}</p></header>
    <nav class="library-categories" aria-label="${locale.categories}">${visibleGroups.map((group) => `<a href="#${group.id}">${escapeHtml(group.title)}</a>`).join('')}</nav>
    <section class="library-feature" aria-labelledby="featured-title"><div><p class="eyebrow">${locale.featured}</p><h2 id="featured-title"><a href="${featured.slug}.html">${escapeHtml(featured.title)}</a></h2><p>${escapeHtml(featured.description)}</p></div><div class="library-feature-action"><span>${escapeHtml(featured.status)} · ${featured.readTime} ${locale.minutes}</span><a class="button button-primary" href="${featured.slug}.html">${locale.read} ${escapeHtml(locale.categoryLabels[featured.category].toLowerCase())}</a></div></section>
    ${visibleGroups.map((group) => `<section class="publication-group" id="${group.id}" aria-labelledby="${group.id}-title"><div class="section-heading"><div><h2 id="${group.id}-title">${escapeHtml(group.title)}</h2></div><p>${escapeHtml(group.description)}</p></div><div class="publication-grid">${catalog.filter((entry) => group.categories.includes(entry.category)).map(publicationCard).join('\n')}</div></section>`).join('\n')}
    <aside class="library-history"><h2>${locale.historyTitle}</h2><p>${locale.history}</p><a href="${locale.assetPrefix}docs/crexe_handoff_docs/README.md">${locale.historyLink}</a></aside>
  </main>`,
  });
}

/** Build standalone reader pages. The caller owns dist cleanup and shared assets. */
export async function buildPublications() {
  const editions = new Map();
  for (const language of supportedLocales) {
    const filename = language === 'en' ? 'publications.en.json' : 'publications.json';
    const catalog = validateCatalog(await readJson(path.join(siteRoot, filename)));
    if (catalog.some((entry) => entry.language && entry.language !== language)) throw new Error(`Wrong content language in ${filename}.`);
    editions.set(language, catalog.map((entry) => ({ ...entry, language })));
  }
  validateEditions(editions);
  const canonicalRepo = await realpath(repoRoot);
  const result = { pages: [], catalog: [], localLinks: [] };
  for (const [language, entries] of editions) {
    const catalog = entries.map((entry) => ({ ...entry,
      sourceAliases: [...editions.values()].map((edition) => edition.find((other) => other.slug === entry.slug).source),
    }));
    const rendered = new Map();
    for (const entry of catalog) {
      const filename = await realpath(path.join(repoRoot, entry.source));
      if (!within(canonicalRepo, filename)) throw new Error(`Publication source escapes repository: ${entry.source}`);
      rendered.set(entry.slug, renderPublication(await readFile(filename, 'utf8'), entry, catalog));
    }
    const enriched = catalog.map((entry) => ({ ...entry,
      wordCount: rendered.get(entry.slug).wordCount,
      readTime: rendered.get(entry.slug).readTime,
      url: publicationPath(language, entry.slug),
    }));
    const output = path.join(distRoot, getLocale(language).library);
    await mkdir(output, { recursive: true });
    await writeFile(path.join(output, 'index.html'), libraryPage(enriched, language), 'utf8');
    for (const entry of enriched) await writeFile(path.join(output, `${entry.slug}.html`), articlePage(entry, rendered.get(entry.slug), enriched), 'utf8');
    result.pages.push(publicationPath(language), ...enriched.map((entry) => entry.url));
    result.catalog.push(...enriched);
    result.localLinks.push(...[...rendered.values()].flatMap((entry) => entry.localLinks));
  }
  return result;
}

export function validateEditions(editions) {
  const primary = editions.get('pt-BR');
  if (!primary) throw new Error('Missing Portuguese catalog.');
  for (const language of supportedLocales) {
    const entries = editions.get(language);
    if (!entries || entries.length !== primary.length) throw new Error(`Incomplete ${language} catalog.`);
    for (const entry of primary) {
      const translation = entries.find((other) => other.slug === entry.slug);
      if (!translation || translation.category !== entry.category) throw new Error(`Missing or mismatched ${language} translation: ${entry.slug}`);
    }
  }
}
