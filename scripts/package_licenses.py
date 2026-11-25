"""Collect dependency license texts for a native package; Python standard library only.

This is a conservative notice inventory, not an SBOM or a legal certification.
Cargo's filtered resolve graph can include build/development-only dependencies.
"""
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
from urllib.parse import urlsplit


SUPPLEMENT_ROOT = Path(__file__).with_name('third_party_licenses')
CRATES_IO_SOURCE = 'registry+https://github.com/rust-lang/crates.io-index'
MAX_TEXT_BYTES = 2 * 1024 * 1024
MAX_HEADER_BYTES = 64 * 1024
NON_TEXT_SUFFIXES = {
    '.rs', '.c', '.cpp', '.h', '.hpp', '.py', '.pyc', '.js', '.ts', '.toml',
    '.json', '.yaml', '.yml', '.lock', '.a', '.lib', '.dll', '.so', '.dylib',
    '.exe', '.ttf', '.otf', '.woff', '.woff2', '.png', '.jpg', '.bin',
}
NOTICE_NAME = re.compile(
    r'(?:^|[._ -])(?:LICEN[CS]ES?|COPYING|COPYRIGHT|NOTICES?|OFL|UFL)(?:$|[._ -])'
    r'|LICEN[CS]E$|^THIRDPARTYNOTICES(?:$|[._ -])', re.IGNORECASE)
# This font's notice uses the font's filename rather than a license keyword.
EXTRA_NOTICE_PATHS = {
    ('epaint_default_fonts', '0.36.2'): ['fonts/Hack-Regular.txt'],
}
# Only these version-reviewed source/data files contribute an initial notice,
# never their implementation or registry data. Other crates require review first.
HEADER_NOTICE_PATHS = {
    ('accesskit', '0.24.1'): {'src/lib.rs': 'slash'},
    ('khronos_api', '3.1.0'): {
        'src/lib.rs': 'slash',
        'api/xml/gl.xml': 'xml',
        'api/xml/wgl.xml': 'xml',
        'api/xml/glx.xml': 'xml',
        'api_egl/api/egl.xml': 'xml',
        'api_angle/scripts/gl_angle_ext.xml': 'xml',
        'api_angle/scripts/egl_angle_ext.xml': 'xml',
        'api_webgl/specs/latest/1.0/webgl.idl': 'slash',
        'api_webgl/specs/latest/2.0/webgl2.idl': 'slash',
    },
}


class NoticeError(ValueError):
    pass


def relative_path(value):
    if not isinstance(value, str) or not value or '\\' in value:
        raise NoticeError('A notice path must be a portable relative path')
    path = PurePosixPath(value)
    if path.is_absolute() or any(
        part in ('', '.', '..') or part.endswith(('.', ' '))
        or re.search(r'[<>:"|?*\x00-\x1f]', part)
        for part in value.split('/')
    ):
        raise NoticeError('A notice path must stay within its package')
    return path


def notice_filename(relative):
    path = PurePosixPath(relative)
    if path.suffix.lower() in NON_TEXT_SUFFIXES and path.name.upper() != 'COPYING.LIB':
        return False
    return bool(NOTICE_NAME.search(path.name)) or any(
        part.lower() in ('licenses', 'licences') for part in path.parts[:-1])


def source_path(root, relative):
    portable = relative_path(relative)
    path = root
    for part in portable.parts:
        path = path / part
        if path.is_symlink() or (
            path.exists() and getattr(path.lstat(), 'st_file_attributes', 0) & 0x400
        ):
            raise NoticeError('Links/reparse points are not accepted as notice sources')
    if not path.resolve().is_relative_to(root.resolve()) or not path.is_file():
        raise NoticeError('Declared notice file is missing or outside its package')
    return path


def read_notice(root, relative):
    path = source_path(root, relative)
    if (path.suffix.lower() in NON_TEXT_SUFFIXES and path.name.upper() != 'COPYING.LIB') or path.stat().st_size > MAX_TEXT_BYTES:
        raise NoticeError('Notice source is not an allowed bounded text file')
    content = path.read_bytes()
    if not content.strip() or b'\0' in content:
        raise NoticeError('Notice source is empty or binary')
    try:
        content.decode('utf-8-sig')
    except UnicodeDecodeError as error:
        raise NoticeError('Notice source needs an explicit text-encoding review') from error
    return content


def read_header_notice(root, relative, kind):
    path = source_path(root, relative)
    with path.open('rb') as source:
        prefix = source.read(MAX_HEADER_BYTES)
    if kind == 'slash':
        lines = []
        terminated = False
        for line in prefix.splitlines(keepends=True):
            stripped = line.strip()
            if stripped and (not stripped.startswith(b'//') or stripped.startswith((b'//!', b'///'))):
                terminated = True
                break
            lines.append(line)
        if not terminated and len(prefix) == MAX_HEADER_BYTES:
            raise NoticeError('Reviewed copyright header exceeds the extraction limit')
        content = b''.join(lines).rstrip(b'\r\n')
    elif kind == 'xml':
        candidates = re.finditer(rb'<comment>.*?</comment>|<!--.*?-->', prefix, re.DOTALL)
        content = next((match.group() for match in candidates
                        if re.search(rb'copyright', match.group(), re.IGNORECASE)), b'')
    else:
        raise NoticeError('Unknown reviewed notice extraction format')
    if not content or b'\0' in content or not re.search(rb'copyright', content, re.IGNORECASE):
        raise NoticeError('Reviewed copyright header is missing; audit the crate again')
    try:
        content.decode('utf-8-sig')
    except UnicodeDecodeError as error:
        raise NoticeError('Reviewed copyright header is not UTF-8') from error
    return content


def validate_provenance(value):
    if not isinstance(value, str) or not value or any(ord(c) < 32 for c in value):
        raise NoticeError('Supplement provenance must be a public HTTPS URL')
    url = urlsplit(value)
    if url.scheme != 'https' or url.username or url.password or url.query or url.fragment:
        raise NoticeError('Supplement provenance must be a public HTTPS URL')
    parts = url.path.strip('/').split('/')
    if url.hostname != 'raw.githubusercontent.com' or len(parts) < 4 or not re.fullmatch(r'[a-f0-9]{40}', parts[2]):
        raise NoticeError('Supplement source must identify an immutable upstream Git commit')


def reachable_packages(metadata):
    graph = metadata.get('resolve') or {}
    root = graph.get('root')
    nodes = {node['id']: node for node in graph.get('nodes', [])}
    packages = {package['id']: package for package in metadata.get('packages', [])}
    if not root or root not in nodes or root not in packages:
        raise NoticeError('Cargo metadata must contain a resolved root package')
    visited, pending = set(), [root]
    while pending:
        package_id = pending.pop()
        if package_id in visited:
            continue
        if package_id not in nodes or package_id not in packages:
            raise NoticeError('Cargo dependency graph is incomplete')
        visited.add(package_id)
        pending.extend(dependency['pkg'] for dependency in nodes[package_id]['deps'])
    return sorted((packages[item] for item in visited if item != root),
                  key=lambda item: (item['name'], item['version'], item['id']))


def load_supplements(root):
    data = json.loads((root / 'index.json').read_text(encoding='utf-8'))
    if data.get('schema') != 1:
        raise NoticeError('Unknown reviewed notice supplement schema')
    result = {}
    for group in data['groups']:
        for package in group['packages']:
            key = (package['name'], package['version'])
            if key in result:
                raise NoticeError('Duplicate reviewed package supplement')
            result[key] = {'license': package['license'], 'files': group['files'],
                           'note': group.get('note'), 'source_url': group.get('source_url')}
    return result


def collect_notices(metadata, host, supplements=None, supplement_root=None):
    """Return package-relative paths -> exact bytes, without local metadata paths."""
    if not re.fullmatch(r'[A-Za-z0-9_.-]+', host):
        raise NoticeError('Invalid native target triple')
    supplement_root = Path(supplement_root or SUPPLEMENT_ROOT)
    if supplements is None:
        supplements = load_supplements(supplement_root)
    output, names, sections = {}, set(), []
    package_keys = set()

    def include(destination, content):
        relative_path(destination)
        if destination.casefold() in names:
            raise NoticeError('Case-insensitive notice destination collision')
        names.add(destination.casefold())
        output[destination] = content

    for package in reachable_packages(metadata):
        name, version = package['name'], package['version']
        if not re.fullmatch(r'[A-Za-z0-9_-]+', name) or not re.fullmatch(r'[A-Za-z0-9_.+-]+', version):
            raise NoticeError('Invalid package name/version in Cargo metadata')
        key = (name, version)
        if key in package_keys:
            raise NoticeError(f'{name} {version}: multiple source packages need explicit disambiguation')
        package_keys.add(key)
        prefix = f'third-party-licenses/{name}-{version}'
        expression = package.get('license')
        declared = package.get('license_file')
        if expression is not None and (not isinstance(expression, str) or not expression.strip()
                                       or any(ord(c) < 32 for c in expression)):
            raise NoticeError(f'{name} {version}: invalid declared license expression')
        if not expression and not declared:
            raise NoticeError(f'{name} {version}: no declared license or license_file')
        root = Path(package['manifest_path']).parent.resolve()
        if not root.is_dir():
            raise NoticeError(f'{name} {version}: package source directory is unavailable')
        selected = set(EXTRA_NOTICE_PATHS.get(key, []))
        if declared:
            selected.add(relative_path(declared).as_posix())
        for directory, directories, files in os.walk(root, followlinks=False):
            for child in directories:
                candidate = Path(directory) / child
                if candidate.is_symlink() or getattr(candidate.lstat(), 'st_file_attributes', 0) & 0x400:
                    raise NoticeError(f'{name} {version}: dependency tree has a link requiring review')
            for filename in files:
                relative = (Path(directory) / filename).relative_to(root).as_posix()
                if notice_filename(relative):
                    selected.add(relative)
        records = []
        for relative in sorted(selected):
            try:
                content = read_notice(root, relative)
            except NoticeError as error:
                raise NoticeError(f'{name} {version}: {error}') from error
            destination = f'{prefix}/{relative}'
            include(destination, content)
            records.append((relative, destination, hashlib.sha256(content).hexdigest(), None))
        headers = HEADER_NOTICE_PATHS.get(key, {})
        if headers and package.get('source') != CRATES_IO_SOURCE:
            raise NoticeError(f'{name} {version}: reviewed headers require the official crates.io source')
        for relative, kind in sorted(headers.items()):
            content = read_header_notice(root, relative, kind)
            destination = f'{prefix}/extracted/{relative}.notice.txt'
            include(destination, content)
            records.append((relative + ' (copyright header only)', destination,
                            hashlib.sha256(content).hexdigest(), None))
        supplement = supplements.get(key)
        if supplement:
            if package.get('source') != CRATES_IO_SOURCE:
                raise NoticeError(f'{name} {version}: reviewed supplement requires the official crates.io source')
            if supplement['license'] != expression:
                raise NoticeError(f'{name} {version}: declared license changed; review its supplement')
            note = supplement.get('note')
            if note is not None and (not isinstance(note, str) or not note.strip()
                                     or any(ord(c) < 32 for c in note)):
                raise NoticeError('Reviewed supplement note must be printable plain text')
            if supplement.get('source_url'):
                validate_provenance(supplement['source_url'])
            for document in supplement['files']:
                validate_provenance(document['url'])
                content = read_notice(supplement_root, document['file'])
                digest = hashlib.sha256(content).hexdigest()
                if digest != document['sha256']:
                    raise NoticeError(f'{name} {version}: reviewed license text hash mismatch')
                original = relative_path(document['original_path']).as_posix()
                destination = f'{prefix}/upstream/{original}'
                include(destination, content)
                records.append((original, destination, digest, document['url']))
        if not records:
            raise NoticeError(f'{name} {version}: no license text found; add a reviewed version-pinned supplement')
        lines = [f'{name} {version}', f'Cargo license: {expression or "custom (declared license_file)"}']
        if supplement and supplement.get('note'):
            lines.append(f'Reviewed supplement limitation: {supplement["note"]}')
        if supplement and supplement.get('source_url'):
            lines.append(f'Reviewed declaration source: {supplement["source_url"]}')
        for original, destination, digest, url in records:
            lines += [f'  Original relative path: {original}', f'  Included file: {destination}', f'  SHA-256: {digest}']
            if url:
                lines.append(f'  Reviewed upstream source: {url}')
        sections.append('\n'.join(lines))
    header = f'''THIRD-PARTY NOTICES / AVISOS DE TERCEIROS

Native target: {host}
Dependency packages in this conservative inventory: {len(sections)}

The engine's MIT license does not change third-party licenses. Preserve the
applicable copyright, license and notice texts supplied in third-party-licenses/.

This inventory follows all packages reachable from the root in Cargo metadata
filtered for the native target, including reachable build/development dependencies.
Some listed components may not be embedded in the engine. This is a conservative
collection of notices, not an SBOM or certification of complete legal compliance.
Cargo license expressions are preserved as declared; alternatives are not silently
chosen or relicensed. Nested/vendored notices and reviewed missing upstream texts
are included with their original relative paths and content hashes.

A licença MIT da engine não altera as licenças de terceiros. Este conjunto
conservador pode incluir dependências de build/teste que não entram no binário;
não é uma certificação de conformidade. Os textos e avisos preservam seus termos.

'''
    output['THIRD-PARTY-NOTICES.txt'] = (header + '\n\n'.join(sections) + '\n').encode('utf-8')
    return dict(sorted(output.items()))
