"""Rebuild or verify the downloadable, deterministic minimal CREXP example."""
import argparse
import io
from pathlib import Path
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'examples/crexp/calculadora'
OUTPUT = ROOT / 'examples/crexp/calculadora.crexp'
NAMES = ('crexp.yaml', 'requirements.md', 'tests.md')


def build_example():
    output = io.BytesIO()
    with zipfile.ZipFile(output, 'w', compression=zipfile.ZIP_STORED) as archive:
        for name in NAMES:
            # Git checkouts may use CRLF. The package has one canonical byte form.
            content = (SOURCE / name).read_text(encoding='utf-8').encode('utf-8')
            entry = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            entry.create_system = 3
            entry.external_attr = 0o100644 << 16
            entry.compress_type = zipfile.ZIP_STORED
            archive.writestr(entry, content)
    return output.getvalue()


def verified_example():
    expected = build_example()
    if not OUTPUT.is_file() or OUTPUT.read_bytes() != expected:
        raise SystemExit('CREXP example is stale or missing. Run python scripts/crexp_example.py and commit its output.')
    return expected


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='Verify without writing files.')
    args = parser.parse_args()
    if args.check:
        data = verified_example()
        print(f'CREXP example matches all three maintained sources ({len(data)} bytes).')
    else:
        data = build_example()
        OUTPUT.write_bytes(data)
        print(f'Wrote {OUTPUT} ({len(data)} bytes).')


if __name__ == '__main__':
    main()
