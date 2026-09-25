"""Build and package a native development snapshot; never publish or install it.

Requires Git, Cargo and Python 3.11+ on the development/CI host only.
The extracted engine is smoke-tested without compilers in PATH.
"""
import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def command(args, **kwargs):
    return subprocess.run(args, cwd=ROOT, check=True, **kwargs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir', type=Path, default=ROOT / 'target/packages')
    args = parser.parse_args()
    command(['git', 'diff', '--exit-code', '--quiet', 'HEAD', '--'])
    untracked = command(['git', 'ls-files', '--others', '--exclude-standard'], capture_output=True).stdout
    if untracked.strip():
        raise SystemExit('Commit the repository changes before packaging a source-linked snapshot.')
    revision = command(['git', 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip()
    command(['cargo', 'build', '--release', '--locked'])
    binary = ROOT / 'target/release' / ('crexe.exe' if os.name == 'nt' else 'crexe')
    with tempfile.TemporaryDirectory(prefix='crexe-package-check-') as check:
        env = dict(os.environ, CREXE_HOME=str(Path(check) / 'home'), PATH='')
        info = command([str(binary), 'doctor'], env=env, capture_output=True, text=True, encoding='utf-8').stdout
        match = re.search(r'^CREXE (\S+) .+ (windows|linux|macos) / ([\w-]+)', info)
        if not match:
            raise SystemExit('Cannot determine the native engine version/platform.')
        version, system, arch = match.groups()
        name = f'crexe-{version}-dev.{revision[:7]}-{system}-{arch}'
        source_url = f'https://github.com/omarceloribeiro/crexe/archive/{revision}.tar.gz'
        files = {
            binary.name: (binary.read_bytes(), 0o755),
            'LICENSE': ((ROOT / 'LICENSE').read_bytes(), 0o644),
            'config.example.toml': ((ROOT / 'config.example.toml').read_bytes(), 0o644),
            'examples/calculadora.crexe': ((ROOT / 'examples/plaintext/calculadora.crexe').read_bytes(), 0o644),
            'examples/regra-de-tres.crexe': ((ROOT / 'examples/markdown/regra-de-tres.crexe').read_bytes(), 0o644),
            'SOURCE.txt': ((source_url + '\n').encode(), 0o644),
        }
        if system == 'windows':
            install = '@echo off\r\nsetlocal\r\n"%~dp0crexe.exe" install\r\nif errorlevel 1 exit /b %errorlevel%\r\n"%~dp0crexe.exe" associate\r\nexit /b %errorlevel%\r\n'
            files['install.cmd'] = (install.encode(), 0o644)
            install_help = 'Execute install.cmd no terminal. Instala em %LOCALAPPDATA%\\CREXE\\releases\\v1 e associa .crexe ao caminho instalado. O Windows pode pedir para escolher o aplicativo padrão.'
        else:
            install = '#!/bin/sh\nset -eu\ncd -- "$(dirname -- "$0")"\nexec ./crexe install\n'
            files['install.sh'] = (install.encode(), 0o755)
            install_help = 'Execute ./install.sh no terminal. A associação gráfica e a janela de espera ainda não estão implementadas neste OS; use a CLI.'
        invocation = '.\\crexe.exe' if system == 'windows' else './crexe'
        readme = f'''# CREXE — snapshot de desenvolvimento

Versão do pacote: {version}; commit: {revision}; destino: {system}/{arch}.
Este pacote serve para validação. Não é uma release estável ou um instalador dos SDKs dos apps.

{install_help}

A engine compilada não precisa de Rust, Python, Docker ou WSL para executar.
Para gerar programas, prepare o provider e o SDK do perfil:
- Ollama local com gemma4:12b é o default. A engine não baixa modelos nem troca automaticamente para OpenAI.
- Windows: .NET 8 SDK no PATH para Windows Forms, ou MinGW-w64 g++ para --profile cpp-win32.
- macOS: Apple Clang/SDK Cocoa; perfil experimental objc-cocoa.
- Linux: GCC, Make, pkg-config e GTK3 development; perfil experimental c-gtk.

Na pasta extraída do pacote, execute:

```
{invocation} doctor
{invocation} doctor --check
{invocation} inspect examples/calculadora.crexe
{invocation} examples/calculadora.crexe
```

Os três primeiros comandos não chamam a IA. O último gera, compila e executa o programa.
A instalação não adiciona a engine ao PATH. Os comandos acima usam o binário extraído;
para chamar a cópia instalada, use o caminho completo mostrado por doctor.
Editar a intenção gera outra revisão; reabrir sem alterações usa o cache validado.

Provider/modelo/credenciais e limites são configurados localmente, usando config.example.toml como modelo.
--config seleciona outro arquivo. Chaves reais não pertencem ao .crexe nem ao exemplo distribuído.
O caminho padrão da configuração aparece em crexe doctor. Nenhuma credencial foi incluída neste pacote.

Build, testes e app rodam com as permissões do usuário. A pasta temporária não é isolamento de segurança.
Não execute receitas/código nos quais você não confia. Windows Sandbox é proibido nesta fase.

Confira a documentação, requisitos e limites: https://github.com/omarceloribeiro/crexe/tree/{revision}
Código-fonte correspondente: {source_url}
A licença da engine está em LICENSE. Ela não define automaticamente a licença de apps gerados.
'''
        files['LEIA-ME.md'] = (readme.encode('utf-8'), 0o644)
        manifest = {'schema': 1, 'status': 'development snapshot', 'version': version,
                    'source_commit': revision, 'source_url': source_url, 'os': system, 'arch': arch,
                    'files': {path: digest(data) for path, (data, _) in sorted(files.items())}}
        files['PACKAGE.json'] = ((json.dumps(manifest, indent=2) + '\n').encode(), 0o644)
        for path, (data, _) in files.items():
            if path != binary.name and re.search(rb'\bsk-(?:proj-)?[A-Za-z0-9_-]{24,}', data):
                raise SystemExit(f'Unexpected credential pattern in package input: {path}')
        output = args.output_dir.resolve()
        output.mkdir(parents=True, exist_ok=True)
        archive_path = output / (name + ('.zip' if system == 'windows' else '.tar.gz'))
        checksum_path = output / (archive_path.name + '.sha256')
        if archive_path.exists() or checksum_path.exists():
            raise SystemExit('Package output already exists; choose a new output directory.')
        with archive_path.open('xb') as raw:
            if system == 'windows':
                with zipfile.ZipFile(raw, 'w', zipfile.ZIP_DEFLATED) as archive:
                    for path, (data, mode) in sorted(files.items()):
                        entry = zipfile.ZipInfo(name + '/' + path)
                        entry.create_system = 3
                        entry.external_attr = (0o100000 | mode) << 16
                        entry.compress_type = zipfile.ZIP_DEFLATED
                        archive.writestr(entry, data)
            else:
                with gzip.GzipFile(fileobj=raw, filename='', mode='wb', mtime=0) as compressed:
                    with tarfile.open(fileobj=compressed, mode='w') as archive:
                        for path, (data, mode) in sorted(files.items()):
                            entry = tarfile.TarInfo(name + '/' + path)
                            entry.size, entry.mode = len(data), mode
                            archive.addfile(entry, io.BytesIO(data))
        unpacked = Path(check) / 'unpacked'
        if system == 'windows':
            with zipfile.ZipFile(archive_path) as archive:
                archive.extractall(unpacked)
        else:
            with tarfile.open(archive_path) as archive:
                archive.extractall(unpacked, filter='data')
        for path, (data, _) in files.items():
            if digest((unpacked / name / path).read_bytes()) != digest(data):
                raise SystemExit('Extracted package hash mismatch.')
        extracted = unpacked / name / binary.name
        command([str(extracted), '--version'], env=env)
        command([str(extracted), 'doctor'], env=env, capture_output=True)
        with checksum_path.open('x', encoding='ascii') as checksum:
            checksum.write(digest(archive_path.read_bytes()) + '  ' + archive_path.name + '\n')
        print(f'Package verified with empty PATH: {archive_path}')
        print(f'Checksum: {checksum_path}')


if __name__ == '__main__':
    main()
