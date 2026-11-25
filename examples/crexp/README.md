# Exemplo CREXP: calculadora

[calculadora.crexp](calculadora.crexp) é um ZIP de especificações aceito pela prévia CREXP. Contém `crexp.yaml`, `requirements.md` e `tests.md`; os três fontes estão na pasta `calculadora/`. É uma entrada para geração, não um aplicativo já compilado. O texto dos documentos está em português.

```powershell
.\target\release\crexe.exe inspect examples/crexp/calculadora.crexp
.\target\release\crexe.exe exec examples/crexp/calculadora.crexp
```

Inspecionar não chama a IA. Gerar usa o provider configurado, envia os documentos como intenção e exige o SDK do perfil. Os cenários de aceitação em Markdown fazem parte da intenção; a engine não transforma automaticamente essa tabela em uma suíte formal. Veja o [guia CREXP](../../docs/publicacoes/crexp.md).

Para reconstruir o ZIP depois de editar os três fontes:

```sh
python scripts/crexp_example.py
python scripts/crexp_example.py --check
```

O script usa entradas sem compressão, ordenação fixa, timestamps fixos e texto normalizado para LF; o resultado é determinístico entre plataformas. O modo `--check` só compara os bytes esperados com o arquivo versionado e falha se o download estiver desatualizado.

As partes de autoria do CREXE deste exemplo seguem a MIT-0 conforme [LICENSE-GENERATED](../../LICENSE-GENERATED).

## English

This is a valid minimal CREXP specification package for a calculator, with Portuguese source documents. It is a generation input, not a compiled application. `inspect` does not call AI; execution sends its specifications to the configured provider and requires the profile's SDK. Markdown acceptance scenarios are intent, not an automatically executed formal test suite. See the [English guide](../../docs/en/publicacoes/crexp.md). Run the commands above from the repository root to regenerate the deterministic ZIP or check that the tracked download matches its maintained sources. CREXE-authored example materials use MIT-0 under the linked output license.
