import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { availablePackages, quickstartFor } from '../quickstart.js';

for (const os of ['windows', 'macos', 'linux']) {
  for (const english of [false, true]) {
    for (const nativePackage of [false, true]) {
      test(`${os} ${english ? 'EN' : 'PT'} ${nativePackage ? 'package' : 'source'} commands and requirements`, () => {
        const instructions = quickstartFor(os, nativePackage, english);
        const executable = os === 'windows'
          ? (nativePackage ? '.\\crexe.exe' : '.\\target\\release\\crexe.exe')
          : (nativePackage ? './crexe' : './target/release/crexe');
        assert.equal(instructions.firstCommand, nativePackage ? `${executable} --version` : 'cargo build --release --locked');
        assert.equal(instructions.configureCommand, `${executable} configure\n${executable} doctor --check`);
        assert.equal(instructions.runCommand, `${executable} ${nativePackage ? 'examples/calculadora.crexe' : 'examples/plaintext/calculadora.crexe'}`);
        assert.equal(instructions.requirements.includes('Rust/Cargo'), !nativePackage);
        assert.match(instructions.requirements, os === 'windows' ? /\.NET 8 SDK/ : os === 'macos' ? /Apple Clang.*Cocoa/ : /GCC.*GTK3/);
        assert.match(instructions.firstTitle, english ? (nativePackage ? /^Extract/ : /^Build/) : (nativePackage ? /^Extraia/ : /^Compile/));
        assert.match(instructions.firstDescription, nativePackage ? /SHA-256/ : /ZIP/);
        assert.match(instructions.runDescription, english ? (nativePackage ? /package/ : /source code/) : (nativePackage ? /pacote/ : /código-fonte/));
        assert.match(instructions.firstCopyLabel, english ? /^Copy / : /^Copiar /);
      });
    }
  }
}

const packageEntry = (name) => ({
  url: `https://example.org/releases/v1.0.0/crexe-${name}.zip`,
  checksumUrl: `https://example.org/releases/v1.0.0/crexe-${name}.zip.sha256`,
});
const partial = {
  version: 'v1.0.0', releaseUrl: 'https://example.org/releases/v1.0.0',
  platforms: { windows: packageEntry('windows'), macos: packageEntry('macos'), linux: { url: null, checksumUrl: null } },
};

test('only complete HTTPS packages enable native instructions in a partial release', () => {
  assert.deepEqual([...availablePackages(partial)], ['windows', 'macos']);
  assert.equal(quickstartFor('linux', availablePackages(partial).has('linux'), false).firstCommand, 'cargo build --release --locked');
  for (const url of [null, '', '/package.zip', 'http://example.org/package.zip', 'javascript:alert(1)', 'https://user:password@example.org/package.zip']) {
    for (const field of ['url', 'checksumUrl']) {
      const manifest = structuredClone(partial);
      manifest.platforms.windows[field] = url;
      assert.deepEqual([...availablePackages(manifest)], ['macos'], `${field}: ${url}`);
    }
  }
  for (const manifest of [null, {}, { ...partial, version: null }, { ...partial, version: ' ' }, { ...partial, releaseUrl: null }]) {
    assert.equal(availablePackages(manifest).size, 0);
  }
  assert.throws(() => quickstartFor('unsupported', true, false), RangeError);
});

test('both static landings retain usable source instructions without JavaScript', async () => {
  for (const english of [false, true]) {
    const html = await readFile(new URL(english ? '../en/index.html' : '../index.html', import.meta.url), 'utf8');
    const source = quickstartFor('windows', false, english);
    for (const [id, expected] of [
      ['first-step-title', source.firstTitle], ['first-step-description', source.firstDescription],
      ['build-command', source.firstCommand], ['configure-command', source.configureCommand],
      ['run-description', source.runDescription], ['run-command', source.runCommand],
      ['os-requirements', source.requirements],
    ]) {
      const content = html.match(new RegExp(`<[^>]+id="${id}"[^>]*>([^<]*)<`))?.[1].replaceAll('\r\n', '\n');
      assert.equal(content, expected, `${english ? 'EN' : 'PT'}: ${id}`);
    }
    assert.match(html, /type="module"[^>]*src="(?:\.\.\/)?main\.js"/);
  }
});

function element(dataset = {}) {
  return {
    dataset, textContent: '', hidden: true, attributes: {}, listeners: {},
    classList: { add() {}, remove() {}, toggle() {} },
    setAttribute(name, value) { this.attributes[name] = value; },
    getAttribute(name) { return this.attributes[name]; },
    addEventListener(name, callback) { this.listeners[name] = callback; },
  };
}

// Exercise main.js's asynchronous catalog update and tab handlers without a
// browser dependency. This is a DOM wiring check, not a visual browser test.
test('a delayed partial catalog refreshes the selected tab and keeps other platforms on source', async () => {
  const previousDocument = globalThis.document;
  const previousFetch = globalThis.fetch;
  try {
    for (const english of [false, true]) {
      const nodes = new Map();
      const node = (selector) => {
        if (!nodes.has(selector)) nodes.set(selector, element());
        return nodes.get(selector);
      };
      const tabs = ['windows', 'macos', 'linux'].map((os) => element({ os }));
      const cards = ['windows', 'macos', 'linux'].map((platform) => {
        const card = element({ platform });
        card.querySelector = (selector) => node(`${platform} ${selector}`);
        return card;
      });
      globalThis.document = {
        documentElement: { lang: english ? 'en' : 'pt-BR', classList: { add() {} } },
        querySelector: node,
        querySelectorAll(selector) {
          if (selector === '[data-os]' || selector === '[data-os], [data-choose-os]') return tabs;
          if (selector === '[data-platform]') return cards;
          return [];
        },
        addEventListener() {},
      };
      let finishDownload;
      const response = new Promise((resolve) => { finishDownload = resolve; });
      globalThis.fetch = (url) => url.pathname.endsWith('/downloads.json') ? response : Promise.resolve({ ok: false });
      await import(`../main.js?quickstart-test=${english}`);
      tabs[1].listeners.click();
      assert.equal(node('#build-command').textContent, 'cargo build --release --locked');
      finishDownload({ ok: true, json: async () => structuredClone(partial) });
      await new Promise((resolve) => setImmediate(resolve));
      assert.equal(tabs[1].getAttribute('aria-pressed'), 'true');
      assert.equal(node('#build-command').textContent, './crexe --version');
      assert.equal(node('#run-command').textContent, './crexe examples/calculadora.crexe');
      assert.equal(node('macos [data-package-link]').hidden, false);
      assert.equal(node('linux [data-package-link]').hidden, true);
      tabs[0].listeners.click();
      assert.equal(node('#build-command').textContent, '.\\crexe.exe --version');
      assert.doesNotMatch(node('#os-requirements').textContent, /Rust/);
      tabs[2].listeners.click();
      assert.equal(node('#build-command').textContent, 'cargo build --release --locked');
      assert.equal(node('#run-command').textContent, './target/release/crexe examples/plaintext/calculadora.crexe');
      assert.match(node('#os-requirements').textContent, /Rust/);
    }
  } finally {
    globalThis.document = previousDocument;
    globalThis.fetch = previousFetch;
  }
});
