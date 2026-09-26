# Pacotes de desenvolvimento

A CI guarda snapshots por host, com retenção de 14 dias. Não cria releases, tags ou downloads no site. A matriz atual compila nativamente Windows, Ubuntu e macOS; a arquitetura fica no nome e no `PACKAGE.json`. Não há cross-compilation implícita nem garantia de binário universal.

Para preparar um pacote local, com o checkout limpo e todos os arquivos de origem commitados:

```sh
python scripts/package-engine.py
```

Python 3.11+, Git, Cargo e linker são ferramentas do **desenvolvedor**. O script recompila em modo release com lockfile, produz `target/packages`, verifica o conteúdo extraído por hash e executa `--version`/`doctor` com PATH vazio. Isso verifica a inicialização sem compiladores no PATH, não substitui uma máquina limpa. Pacotes existentes não são sobrescritos; `--output-dir` escolhe outra pasta.

O pacote contém a engine, script de instalação sem build, exemplos, configuração sem segredo, licença e link fixado ao commit correspondente. Windows usa ZIP; Linux/macOS usam tar.gz com permissões de execução. `PACKAGE.json` registra origem/plataforma/hashes; o arquivo `.sha256` verifica os bytes do arquivo compactado. Hash não é assinatura do autor.

O build Windows MSVC usa `crt-static`, conforme a [referência de Rust](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes). O executável foi inspecionado sem imports de `VCRUNTIME140.dll`/MSVCP; bibliotecas nativas do Windows permanecem. O verificador da CI impede regressão dessa dependência. Builds por script entram na raiz para aplicar `.cargo/config.toml` e o toolchain fixado mesmo quando chamados de outro diretório.

O pacote da engine não inclui Ollama/modelos ou SDKs dos aplicativos. Instalar a engine dispensa Rust e Python; gerar um app continua exigindo o provider e o SDK de seu [perfil](PERFIS_V1.md). O script Windows instala em `releases/v1` e associa a extensão; Linux/macOS têm instalação por CLI, sem associação gráfica implementada.

Os instaladores interativos abrem a configuração nativa ao concluir. O pacote inclui `configure.cmd` no Windows, `configure.command` no macOS ou `configure.sh` no Linux. Eles usam a cópia instalada ou, se ausente, a cópia extraída. `crexe install` continua não interativo, inclusive sem display. A GUI usa egui/eframe com OpenGL; macOS/Linux precisam de sessão gráfica e bibliotecas de desktop disponíveis. Salvar chaves usa o cofre nativo (Secret Service no Linux). Verificar requisitos reais em máquinas limpas permanece parte da aceitação; build em CI não comprova todas as sessões gráficas.

O checkpoint `845e1bc` produziu e verificou os três pacotes na [CI 36096064360](https://github.com/omarceloribeiro/crexe/actions/runs/36096064360): Windows/x86_64, Linux/x86_64 e macOS/aarch64. Os arquivos baixados tiveram checksum e todos os hashes internos conferidos neste host. O binário Windows produzido no GitHub foi instalado em uma raiz de teste e executou `--version`, `doctor` e `inspect` com PATH vazio, a partir de outra pasta. [Registro dos pacotes](validation/2026-09-25-engine-packages.json).

A instalação não altera o PATH. No PowerShell, use `.\crexe.exe` dentro da pasta extraída ou o caminho completo da instalação. Para concluir os ensaios fora do Codex, siga o [roteiro Windows](VALIDACAO_INSTALACAO_WINDOWS.md).

Antes de uma release pública, ainda é necessário validar instalação/atualização fora do ambiente do Codex, primeira execução em máquina limpa, requisitos mínimos de OS/arquitetura, políticas de assinatura/distribuição e os cenários de app declarados estáveis. O ambiente do Codex pode virtualizar caminhos AppData; testes feitos nele não comprovam a instalação percebida pelo Explorer em uma sessão normal.
