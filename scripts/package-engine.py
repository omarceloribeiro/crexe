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
from package_licenses import collect_notices
from crexp_example import verified_example

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
    compiler = command(['rustc', '-vV'], capture_output=True, text=True, encoding='utf-8').stdout
    host = next((line.removeprefix('host: ') for line in compiler.splitlines() if line.startswith('host: ')), None)
    if not host:
        raise SystemExit('Cannot determine the native Cargo target for dependency notices.')
    metadata = json.loads(command(
        ['cargo', 'metadata', '--locked', '--format-version', '1', '--filter-platform', host],
        capture_output=True, text=True, encoding='utf-8').stdout)
    notices = collect_notices(metadata, host)
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
            'LICENSE-GENERATED': ((ROOT / 'LICENSE-GENERATED').read_bytes(), 0o644),
            'config.example.toml': ((ROOT / 'config.example.toml').read_bytes(), 0o644),
            'examples/calculadora.crexe': ((ROOT / 'examples/plaintext/calculadora.crexe').read_bytes(), 0o644),
            'examples/regra-de-tres.crexe': ((ROOT / 'examples/markdown/regra-de-tres.crexe').read_bytes(), 0o644),
            'examples/calculadora.crexp': (verified_example(), 0o644),
            'examples/primeira-leitura.md': ('# Primeiro documento local\n\n- Este item veio do arquivo selecionado.\n\nAbra outra-leitura.md para reutilizar o leitor.\n'.encode('utf-8'), 0o644),
            'examples/outra-leitura.md': ('# Outro documento, mesmo leitor\n\n- Agora esta janela deve mostrar o segundo arquivo.\n\nEdite este texto externamente e reabra para conferir a leitura do disco.\n'.encode('utf-8'), 0o644),
            'SOURCE.txt': ((source_url + '\n').encode(), 0o644),
        }
        if files.keys() & notices.keys():
            raise SystemExit('Dependency notice paths collide with engine package files.')
        files.update({path: (content, 0o644) for path, content in notices.items()})
        if system == 'windows':
            install = '@echo off\r\nsetlocal\r\n"%~dp0crexe.exe" install\r\nif errorlevel 1 exit /b %errorlevel%\r\n"%~dp0crexe.exe" associate\r\nif errorlevel 1 exit /b %errorlevel%\r\ncall "%~dp0configure.cmd"\r\nexit /b %errorlevel%\r\n'
            files['install.cmd'] = (install.encode(), 0o644)
            configure = '@echo off\r\nsetlocal\r\nset "CREXE_CONFIG_ENGINE=%LOCALAPPDATA%\\CREXE\\releases\\v1\\crexe.exe"\r\nif defined CREXE_HOME set "CREXE_CONFIG_ENGINE=%CREXE_HOME%\\releases\\v1\\crexe.exe"\r\nif not exist "%CREXE_CONFIG_ENGINE%" set "CREXE_CONFIG_ENGINE=%~dp0crexe.exe"\r\n"%CREXE_CONFIG_ENGINE%" configure %*\r\nexit /b %errorlevel%\r\n'
            files['configure.cmd'] = (configure.encode(), 0o644)
            unassociate = '@echo off\r\nsetlocal\r\nset "CREXE_REMOVE_ENGINE=%LOCALAPPDATA%\\CREXE\\releases\\v1\\crexe.exe"\r\nif defined CREXE_HOME set "CREXE_REMOVE_ENGINE=%CREXE_HOME%\\releases\\v1\\crexe.exe"\r\nif not exist "%CREXE_REMOVE_ENGINE%" set "CREXE_REMOVE_ENGINE=%~dp0crexe.exe"\r\n"%CREXE_REMOVE_ENGINE%" unassociate\r\nexit /b %errorlevel%\r\n'
            files['unassociate.cmd'] = (unassociate.encode(), 0o644)
            install_help = 'Execute install.cmd. Instala em %LOCALAPPDATA%\\CREXE\\releases\\v1, associa .crexe, .crexp e .crexb, registra Gerar com CREXE para um arquivo por vez e abre Configurações. No Windows 11, procure também Mostrar mais opções. O menu genérico preserva os aplicativos padrão dos documentos; arquivos .zip não são associados globalmente. Reinstalar preserva suas preferências. configure.cmd reabre a tela; unassociate.cmd remove as associações e o menu pertencentes a esta instalação. O Windows pode pedir para escolher o aplicativo padrão dessas extensões.'
        else:
            launcher = 'configure.command' if system == 'macos' else 'configure.sh'
            default_root = '$HOME/Library/Application Support/CREXE' if system == 'macos' else '${XDG_DATA_HOME:-$HOME/.local/share}/crexe'
            configure = f'#!/bin/sh\nset -eu\nscript_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)\nengine="${{CREXE_HOME:-{default_root}}}/releases/v1/crexe"\nif [ ! -x "$engine" ]; then engine="$script_dir/crexe"; fi\nexec "$engine" configure "$@"\n'
            files[launcher] = (configure.encode(), 0o755)
            install = f'#!/bin/sh\nset -eu\ncd -- "$(dirname -- "$0")"\n./crexe install\nexec ./{launcher}\n'
            files['install.sh'] = (install.encode(), 0o755)
            if system == 'macos':
                files['install.command'] = (install.encode(), 0o755)
            install_help = f'Execute ./install.sh em uma sessão gráfica. Instala e abre Configurações; {launcher} reabre a tela. Sem desktop, use ./crexe install (não abre UI). Para Gerar com CREXE, use o botão Selecionar arquivo e gerar programa… na janela. Esta prévia não registra menu do Finder ou do gerenciador Linux; associações gráficas de .crexe/.crexp/.crexb continuam indisponíveis neste OS. Use a CLI para receitas, pacotes e bundles.'
        invocation = '.\\crexe.exe' if system == 'windows' else './crexe'
        readme = f'''# CREXE — snapshot de desenvolvimento

Versão do pacote: {version}; commit: {revision}; destino: {system}/{arch}.
Este pacote serve para validação. Não é uma release estável ou um instalador dos SDKs dos apps.

{install_help}

A engine compilada não precisa de Rust, Python, Docker ou WSL para executar.
Para gerar programas, prepare o provider e o SDK do perfil:
- Ollama local com gemma4:12b é o default. OpenAI e DeepSeek podem ser selecionados em Configurações, com API key no cofre. A engine não baixa modelos nem troca automaticamente de provider.
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

## CREXP: várias especificações em um arquivo

examples/calculadora.crexp é um ZIP com crexp.yaml, requirements.md e tests.md.
É uma receita para gerar, não um aplicativo compilado. Os documentos são em português.

```
{invocation} inspect examples/calculadora.crexp
{invocation} exec examples/calculadora.crexp
```

Inspecionar não chama a IA. Executar consolida os Markdown e usa o provider,
SDK e tentativas normais. Com provider remoto, os documentos SDD são enviados
como intenção. A tabela de aceitação no pacote não é uma suíte formal executável.
A prévia não suporta assets, agentes ou resolução semântica de conflitos.

## CREXB: compartilhar e abrir uma revisão aceita

Depois de gerar a calculadora, exporte o snapshot aceito:

```
{invocation} bundle create examples/calculadora.crexp --target current --output calculadora.crexb
{invocation} bundle inspect calculadora.crexb
{invocation} bundle verify calculadora.crexb
{invocation} bundle run calculadora.crexb
```

A exportação requer uma revisão aceita e íntegra com snapshot da receita original;
cache antigo sem snapshot precisa de regeneração explícita uma vez. Editar um CREXP
mantém a linhagem pelo caminho original: se a nova tentativa falhar, a última revisão
aceita e sua receita original ainda podem ser exportadas.
Não existe download de um aplicativo CREXB pré-gerado neste pacote da engine.

Abertura compatível não chama IA, SDK ou compilador. Exige a mesma plataforma/
arquitetura e runtimes já instalados: .NET 8 para apps dependentes do framework,
ou bibliotecas do sistema para perfis nativos. OpenGL e comandos YAML customizados
ficam fora desta prévia. O código-fonte e referências aos autotestes vão no bundle;
isso não prova aprovação humana, segurança ou aceitação visual do aplicativo.

Se o target for incompatível, extraia a receita e gere explicitamente no destino:

```
{invocation} bundle extract-recipe calculadora.crexb --output recuperada.crexp
{invocation} exec recuperada.crexp
```

A última linha exige provider e SDK. Escolha .crexe se a receita inspecionada for
textual. O bundle original não muda. Hashes verificam integridade, não autoria;
assinaturas não são suportadas. Dados do app ficam separados do payload em cache.
Não execute bundles desconhecidos: o programa roda no host, sem sandbox.
Leia CREXP-CREXB.en.md para o guia equivalente em inglês.

## Gerar um programa para abrir um arquivo

Escolha **Selecionar arquivo e gerar programa…** na janela da engine, ou use o
menu **Gerar com CREXE** do Windows após a instalação. A CLI equivalente é:

```
{invocation} open-generating -- examples/primeira-leitura.md
{invocation} open-generating -- examples/outra-leitura.md
```

Na primeira preparação do tipo, uma única chamada à LLM formula a intenção do
leitor a partir somente da extensão. O planejamento não desconta uma tentativa
de código: o padrão permite um plano e até três tentativas de código, sujeitos
aos limites de tokens e tempo. Seguem build, autotestes e eventuais reparos.
O segundo documento reutiliza o leitor compatível aprovado,
sem outra chamada de planejamento ou geração quando o cache continua válido.
Extensões desconhecidas válidas também entram nesse fluxo. Arquivos sem extensão,
com extensão inválida ou .crexe/.crexp/.crexb recebem aviso sem chamada ao modelo;
receitas, pacotes e bundles usam sua abertura normal. Formatos opacos podem resultar em leitura de texto
ou inspeção hexadecimal identificada.

O documento selecionado abre como dado em runtime: seu nome, caminho e conteúdo
não são enviados ao modelo nem incorporados ao projeto exportado. O contrato
orienta a preservar o original; ele não transforma o processo gerado em sandbox.
O programa deve apresentar conteúdo ou erro legível e sinalizar leitura limitada.
As duas receitas .crexe anteriores continuam usando o fluxo normal de receitas.
Gerar o primeiro leitor exige provider/modelo e SDK configurados, e um provider
remoto pode cobrar pelo planejamento, geração e reparos.

No Windows, o segundo menu **Gerar com CREXE e Manter como Programa Padrão**
preserva o leitor aprovado fora do cache e oferece a escolha nas Configurações.
Confira o leitor e confirme o padrão para a extensão no próprio Windows.
A abertura pelo leitor mantido dispensa modelo, credenciais e compilação, mesmo
após limpar o cache de geração. É uma prévia somente leitura para documentos.
Use `reader-defaults list`, `reader-defaults choose --extension xyz` ou
`reader-defaults forget --extension xyz` para gerenciar. Forget remove o candidato
e o índice, mas conserva objetos para não interromper leitores em execução.

Use Configurações para escolher provider/modelo e salvar a API key no cofre do sistema.
config.example.toml é apenas um exemplo: editá-lo não altera a configuração ativa.
Na tela, importe o TOML editado, revise e salve. Reinstalar preserva as escolhas já salvas.
--config seleciona outro arquivo. O caminho ativo aparece na tela e em crexe doctor.
Nenhuma credencial foi incluída neste pacote. Em Linux sem cofre/desktop, a configuração por ambiente continua disponível.

Build, testes e app rodam com as permissões do usuário. A pasta temporária não é isolamento de segurança.
Não execute receitas/código nos quais você não confia. Windows Sandbox é proibido nesta fase.

Confira a documentação, requisitos e limites: https://github.com/omarceloribeiro/crexe/tree/{revision}
Código-fonte correspondente: {source_url}
A engine usa MIT (LICENSE); seus avisos se aplicam a cópias da própria engine.
LICENSE-GENERATED descreve a permissão MIT-0 para os materiais de saída fornecidos
pelo CREXE, incluindo as receitas oficiais. Esses materiais MIT-0 podem ser
redistribuídos sem manter o aviso ou atribuir crédito ao CREXE.
Você escolhe a licença do seu aplicativo, inclusive proprietária ou comercial; o CREXE
não exige publicar seus fontes nem dar crédito ao CREXE. Dependências, receitas de
terceiros, assets e termos do provider podem ter condições próprias. A permissão
dos materiais CREXE não licencia automaticamente o aplicativo inteiro.
THIRD-PARTY-NOTICES.txt e third-party-licenses/ preservam os avisos e textos das
dependências. A licença MIT da engine não altera esses termos. O conjunto é
conservador: pode incluir dependências de build/teste não incorporadas ao binário
e não representa certificação de conformidade de todas as licenças.
'''
        files['LEIA-ME.md'] = (readme.encode('utf-8'), 0o644)
        formats_english = f'''# CREXP and CREXB — minimal preview

Development snapshot {version}, commit {revision}, target {system}/{arch}.
These previews are a limited subset of the complete formats, not a stable release.

## Generate from several specifications

examples/calculadora.crexp is a ZIP containing crexp.yaml, requirements.md, and
tests.md. Its documents are in Portuguese. It is generation input, not a compiled app.

```
{invocation} inspect examples/calculadora.crexp
{invocation} exec examples/calculadora.crexp
```

Inspection does not call AI. Execution consolidates the Markdown documents and
uses the configured provider, profile SDK, and normal repair attempts. With a
remote provider, the SDD documents are sent as intent. Markdown acceptance scenarios
are not a formal executable test suite. Assets, agents, and semantic conflict
resolution are outside this preview.

## Export and open the accepted result

After generation succeeds:

```
{invocation} bundle create examples/calculadora.crexp --target current --output calculadora.crexb
{invocation} bundle inspect calculadora.crexb
{invocation} bundle verify calculadora.crexb
{invocation} bundle run calculadora.crexb
```

Export requires an intact accepted revision with the original recipe snapshot.
Old caches without that snapshot require an explicit regeneration once. Editing a
CREXP preserves lineage through the original path: if the new attempt fails, the
last accepted revision and its original recipe can still be exported.
No pre-generated CREXB application is included in this engine package.

Opening a compatible bundle calls no AI, SDK, or compiler. It requires the same
platform/architecture and installed runtimes: .NET 8 for framework-dependent apps,
or system libraries for native profiles. OpenGL and custom YAML commands are
outside this preview. Sources and self-test references are included in bundles;
they do not prove human acceptance, security, or visual application validation.

For an incompatible target, explicitly extract and regenerate the included recipe:

```
{invocation} bundle extract-recipe calculadora.crexb --output recovered.crexp
{invocation} exec recovered.crexp
```

The last line requires a provider and SDK. Use .crexe if inspection identifies a
textual recipe. The original bundle stays unchanged. Hashes verify integrity, not
authorship; signatures are unsupported. App data is separate from cached payloads.
Run only trusted bundles: programs execute on the host without a sandbox.

Windows install/associate registers .crexe, .crexp, and .crexb for double-click
opening, without globally associating .zip. macOS/Linux use the CLI in this preview.
Generate with CREXE rejects all three recipe/package/bundle extensions. For ordinary
documents that feature uses only the extension to plan a reader; it does not send
the document's name, path, contents, or metadata to the model. This differs from
explicit CREXP execution, where specifications are the recipe sent for generation.

On Windows, **Gerar com CREXE e Manter como Programa Padrão** keeps the accepted
reader outside the generation cache and offers Windows default-app settings.
Inspect the reader and confirm the extension-wide choice in Windows. Its launcher
needs no model, generation credentials, or compiler, even after clearing that
cache. This is a read-only document preview. Use `reader-defaults list`,
`reader-defaults choose --extension xyz`, or `reader-defaults forget --extension xyz`.
Forget removes the candidate and index while retaining objects used by running apps.

The engine uses MIT (LICENSE). CREXE-authored output materials and official examples
use MIT-0 (LICENSE-GENERATED). You choose your app's license, subject to applicable
third-party and provider terms. See LEIA-ME.md for the complete installation guide.
'''
        files['CREXP-CREXB.en.md'] = (formats_english.encode('utf-8'), 0o644)
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
        command([str(extracted), 'inspect', str(unpacked / name / 'examples/calculadora.crexe')], env=env, capture_output=True)
        command([str(extracted), 'inspect', str(unpacked / name / 'examples/calculadora.crexp')], env=env, capture_output=True)
        command([str(extracted), 'bundle', '--help'], env=env, capture_output=True)
        command([str(extracted), 'open-generating', '--help'], env=env, capture_output=True)
        with checksum_path.open('x', encoding='ascii') as checksum:
            checksum.write(digest(archive_path.read_bytes()) + '  ' + archive_path.name + '\n')
        print(f'Package verified with empty PATH: {archive_path}')
        print(f'Checksum: {checksum_path}')


if __name__ == '__main__':
    main()
