import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from package_licenses import (
    collect_notices, load_supplements, read_header_notice, read_notice, validate_provenance,
    NoticeError, CRATES_IO_SOURCE, MAX_HEADER_BYTES, SUPPLEMENT_ROOT,
)


class NoticeCollectionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.metadata = {'packages': [], 'resolve': {'root': 'private-root', 'nodes': []}}
        self.add_package('engine', '1.0.0', {}, package_id='private-root', attached=False)

    def add_package(self, name, version, files, license='MIT', license_file=None,
                    package_id=None, attached=True):
        package_id = package_id or f'private-local-source/{name}/{version}'
        directory = self.root / f'{name}-{version}'
        directory.mkdir()
        (directory / 'Cargo.toml').write_text('[package]\n', encoding='utf-8')
        for filename, content in files.items():
            path = directory / filename
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
        self.metadata['packages'].append({
            'id': package_id, 'name': name, 'version': version,
            'manifest_path': str(directory / 'Cargo.toml'),
            'license': license, 'license_file': license_file,
            'source': CRATES_IO_SOURCE,
            'repository': 'https://secret-user:secret-password@example.org/private',
        })
        self.metadata['resolve']['nodes'].append({'id': package_id, 'deps': []})
        if attached:
            self.metadata['resolve']['nodes'][0]['deps'].append(
                {'pkg': package_id, 'dep_kinds': [{'kind': 'build', 'target': None}]})
        return directory

    def collect(self, **kwargs):
        return collect_notices(self.metadata, 'x86_64-pc-windows-msvc',
                               supplements=kwargs.pop('supplements', {}), **kwargs)

    def test_recursive_notices_alternatives_fonts_and_no_source_or_local_metadata(self):
        supplied = {
            'LICENSE-MIT': b'Copyright Ada\r\nMIT terms\r\n',
            'LICENSE-APACHE': b'Apache license terms\n',
            'vendor/library/COPYING.LIB': b'Library copyright\n',
            'fonts/UBUNTU_FONT_LICENCE.txt': b'Ubuntu Font Licence\n',
            'fonts/emoji-icon-font-mit-license.txt': b'Emoji copyright and license\n',
            'vendor/COPYRIGHT': b'Copyright Other Author\n',
            'THIRDPARTYNOTICES': b'Additional notices\n',
            'UNLICENSE': b'Public domain dedication\n',
            'licenses/BSD-3-Clause.txt': b'BSD terms\n',
        }
        self.add_package('example', '1.2.3', supplied | {
            'src/copying.rs': b'fn source_code() {}',
            'libwinapi_oemlicense.a': b'archive',
            'config.json': b'{"api_key":"private-value"}',
        }, license='MIT OR Apache-2.0')
        self.add_package('unreachable', '1.0.0', {}, attached=False)
        files = self.collect()
        self.assertEqual(len(files), len(supplied) + 1)
        for name, content in supplied.items():
            self.assertEqual(files[f'third-party-licenses/example-1.2.3/{name}'], content)
        notice = files['THIRD-PARTY-NOTICES.txt'].decode()
        self.assertIn('Cargo license: MIT OR Apache-2.0', notice)
        self.assertIn('build/development dependencies', notice)
        for private in [str(self.root), 'private-local-source', 'secret-user', 'secret-password', 'private-value', 'unreachable']:
            self.assertNotIn(private, notice)
        self.assertEqual(files, self.collect())

    def test_known_font_filename_is_collected_without_copying_font_binary(self):
        self.add_package('epaint_default_fonts', '0.36.2', {
            'fonts/Hack-Regular.txt': b'Copyright font authors; font license\n',
            'fonts/Hack-Regular.ttf': b'font binary',
            'fonts/OFL.txt': b'Open Font License\n',
        })
        files = self.collect()
        self.assertIn('third-party-licenses/epaint_default_fonts-0.36.2/fonts/Hack-Regular.txt', files)
        self.assertFalse(any(name.endswith('.ttf') for name in files))

    def test_declared_custom_license_file_and_missing_declarations(self):
        self.add_package('custom', '1.0.0', {'legal/terms.txt': b'Custom copyright terms\n'},
                         license=None, license_file='legal/terms.txt')
        self.assertIn('third-party-licenses/custom-1.0.0/legal/terms.txt', self.collect())
        self.metadata['packages'][-1]['license_file'] = None
        with self.assertRaisesRegex(NoticeError, 'no declared license'):
            self.collect()

    def test_unsafe_missing_and_nontext_declared_license_files_fail(self):
        self.add_package('custom', '1.0.0', {'LICENSE': b'License\n'})
        for declared in ['../outside.txt', '/outside.txt', 'C:/outside.txt', 'missing.txt', 'Cargo.toml']:
            self.metadata['packages'][-1]['license_file'] = declared
            with self.subTest(declared=declared), self.assertRaises(NoticeError):
                self.collect()

    def test_missing_text_is_not_silently_replaced_by_spdx_expression(self):
        self.add_package('missing', '2.0.0', {}, license='MIT OR Apache-2.0')
        with self.assertRaisesRegex(NoticeError, 'no license text found'):
            self.collect()

    def test_reviewed_supplement_is_version_expression_and_hash_checked(self):
        self.add_package('missing', '2.0.0', {}, license='MIT OR Apache-2.0')
        content = b'Copyright Verified Author\nMIT permission text\n'
        supplements_root = self.root / 'reviewed'
        supplements_root.mkdir()
        (supplements_root / 'LICENSE-MIT').write_bytes(content)
        document = {'file': 'LICENSE-MIT', 'original_path': 'LICENSE-MIT',
                    'sha256': hashlib.sha256(content).hexdigest(),
                    'url': 'https://raw.githubusercontent.com/example/repository/' + 'a' * 40 + '/LICENSE-MIT'}
        supplements = {('missing', '2.0.0'): {'license': 'MIT OR Apache-2.0', 'files': [document]}}
        files = self.collect(supplements=supplements, supplement_root=supplements_root)
        self.assertEqual(files['third-party-licenses/missing-2.0.0/upstream/LICENSE-MIT'], content)
        for source in [None, 'git+https://example.org/fork', 'registry+https://example.org/private']:
            self.metadata['packages'][-1]['source'] = source
            with self.subTest(source=source), self.assertRaisesRegex(NoticeError, 'official crates.io source'):
                self.collect(supplements=supplements, supplement_root=supplements_root)
        self.metadata['packages'][-1]['source'] = CRATES_IO_SOURCE
        for field, invalid in [('sha256', '0' * 64), ('file', '../outside.txt'),
                               ('url', 'https://raw.githubusercontent.com/example/repository/main/LICENSE-MIT')]:
            original = document[field]
            document[field] = invalid
            with self.subTest(field=field), self.assertRaises(NoticeError):
                self.collect(supplements=supplements, supplement_root=supplements_root)
            document[field] = original
        supplements[('missing', '2.0.0')]['license'] = 'Apache-2.0'
        with self.assertRaisesRegex(NoticeError, 'declared license changed'):
            self.collect(supplements=supplements, supplement_root=supplements_root)

    def test_explicit_standard_text_limitation_and_declaration_provenance_are_preserved(self):
        self.add_package('reviewed', '1.0.0', {})
        supplements_root = self.root / 'reviewed-notices'
        supplements_root.mkdir()
        content = b'MIT standard text, not an invented copyright notice.\n'
        (supplements_root / 'MIT.txt').write_bytes(content)
        base = 'https://raw.githubusercontent.com/example/repository/' + 'a' * 40
        data = {'schema': 1, 'groups': [{
            'packages': [{'name': 'reviewed', 'version': '1.0.0', 'license': 'MIT'}],
            'note': 'Standard text accompanies an audited declaration; upstream has no attribution file.',
            'source_url': base + '/Cargo.toml',
            'files': [{'file': 'MIT.txt', 'original_path': 'text/MIT.txt',
                       'url': base + '/text/MIT.txt', 'sha256': hashlib.sha256(content).hexdigest()}],
        }]}
        (supplements_root / 'index.json').write_text(json.dumps(data), encoding='utf-8')
        supplements = load_supplements(supplements_root)
        files = self.collect(supplements=supplements, supplement_root=supplements_root)
        notice = files['THIRD-PARTY-NOTICES.txt'].decode()
        self.assertIn(data['groups'][0]['note'], notice)
        self.assertIn(base + '/Cargo.toml', notice)
        supplements[('reviewed', '1.0.0')]['source_url'] = base.replace('a' * 40, 'main') + '/Cargo.toml'
        with self.assertRaisesRegex(NoticeError, 'immutable'):
            self.collect(supplements=supplements, supplement_root=supplements_root)

    def test_reviewed_source_headers_exclude_code_and_keep_xml_and_idl_notices(self):
        directory = self.add_package('accesskit', '0.24.1', {
            'LICENSE-MIT': b'MIT terms\n',
            'src/lib.rs': b'// Copyright AccessKit Authors\r\n// License terms\r\n\r\n'
                          b'// Copyright Chromium Authors\r\n// BSD terms\r\n\r\n'
                          b'//! Source documentation\r\npub fn implementation() {}\r\n',
            'api.xml': b'<?xml version="1.0"?><registry><comment>Copyright XML authors\n'
                       b'Permission and conditions\n</comment><commands>source data</commands>',
            'egl.xml': b'<?xml version="1.0"?><!-- Copyright EGL authors\nPermission terms -->'
                       b'<registry>data</registry>',
            'webgl.idl': b'// Generated file\n\n// Copyright IDL authors\n// License terms\n\n'
                         b'typedef unsigned long GLenum;\n',
        })
        files = self.collect()
        extracted = files['third-party-licenses/accesskit-0.24.1/extracted/src/lib.rs.notice.txt']
        self.assertIn(b'Copyright Chromium Authors', extracted)
        self.assertIn(b'\r\n', extracted)
        self.assertNotIn(b'implementation', extracted)
        self.assertNotIn(b'Source documentation', extracted)
        self.assertIn('copyright header only', files['THIRD-PARTY-NOTICES.txt'].decode())
        for filename, kind, absent in [('api.xml', 'xml', b'source data'),
                                       ('egl.xml', 'xml', b'<registry>'),
                                       ('webgl.idl', 'slash', b'typedef')]:
            content = read_header_notice(directory, filename, kind)
            self.assertIn(b'Copyright', content)
            self.assertNotIn(absent, content)
        self.metadata['packages'][-1]['source'] = None
        with self.assertRaisesRegex(NoticeError, 'official crates.io source'):
            self.collect()
        (directory / 'src/lib.rs').write_text('pub fn code_without_notice() {}', encoding='utf-8')
        with self.assertRaisesRegex(NoticeError, 'header is missing'):
            read_header_notice(directory, 'src/lib.rs', 'slash')
        with self.assertRaises(NoticeError):
            read_header_notice(directory, '../outside.rs', 'slash')
        (directory / 'src/lib.rs').write_bytes(b'// Copyright\n' + b'// comment\n' * MAX_HEADER_BYTES)
        with self.assertRaisesRegex(NoticeError, 'exceeds the extraction limit'):
            read_header_notice(directory, 'src/lib.rs', 'slash')

    def test_repository_supplement_bytes_match_reviewed_immutable_sources(self):
        supplements = load_supplements(SUPPLEMENT_ROOT)
        self.assertTrue(supplements)
        for key, supplement in supplements.items():
            with self.subTest(package=key):
                self.assertTrue(supplement['license'])
                self.assertTrue(supplement['files'])
                if supplement.get('source_url'):
                    validate_provenance(supplement['source_url'])
                for document in supplement['files']:
                    validate_provenance(document['url'])
                    content = read_notice(SUPPLEMENT_ROOT, document['file'])
                    self.assertEqual(hashlib.sha256(content).hexdigest(), document['sha256'])

    def test_binary_notice_and_duplicate_package_names_fail(self):
        directory = self.add_package('bad', '1.0.0', {'LICENSE': b'\x00binary'})
        with self.assertRaisesRegex(NoticeError, 'binary'):
            self.collect()
        (directory / 'LICENSE').write_bytes(b'License terms\n')
        duplicate = dict(self.metadata['packages'][-1], id='another-source')
        self.metadata['packages'].append(duplicate)
        self.metadata['resolve']['nodes'].append({'id': duplicate['id'], 'deps': []})
        self.metadata['resolve']['nodes'][0]['deps'].append({'pkg': duplicate['id']})
        with self.assertRaisesRegex(NoticeError, 'disambiguation'):
            self.collect()


if __name__ == '__main__':
    unittest.main()
