# Pacotes de desenvolvimento

A CI guarda snapshots por host, com retenção de 14 dias. Não cria releases, tags ou downloads no site. A matriz atual compila nativamente Windows, Ubuntu e macOS; a arquitetura fica no nome e no `PACKAGE.json`. Não há cross-compilation implícita nem garantia de binário universal.

Para preparar um pacote local, com o checkout limpo e todos os arquivos de origem commitados:

```sh
python scripts/package-engine.py
```

Python 3.11+, Git, Cargo e linker são ferramentas do **desenvolvedor**. O script recompila em modo release com lockfile, produz `target/packages`, verifica o conteúdo extraído por hash e executa `--version`, `doctor`, `inspect` de uma receita incluída e `open-generating --help` com PATH vazio. Isso verifica a inicialização e os comandos sem compiladores no PATH, não substitui uma máquina limpa nem gera um leitor. Pacotes existentes não são sobrescritos; `--output-dir` escolhe outra pasta.

O pacote contém a engine, script de instalação sem build, exemplos, configuração sem segredo, licença e link fixado ao commit correspondente. Windows usa ZIP; Linux/macOS usam tar.gz com permissões de execução. `PACKAGE.json` registra origem/plataforma/hashes; o arquivo `.sha256` verifica os bytes do arquivo compactado. Hash não é assinatura do autor.

Os pacotes nativos também incluem `THIRD-PARTY-NOTICES.txt` e `third-party-licenses/`, com textos de licença, origens, versões e hashes. A coleta usa os metadados Cargo filtrados para o host, incluindo conservadoramente dependências de build e desenvolvimento; textos suplementares fixados por versão em `scripts/third_party_licenses/` e seu mapa associado preservam avisos ausentes dos pacotes de origem. O inventário não afirma que todos os componentes listados estejam incorporados ao binário nem certifica conformidade jurídica completa. As dependências mantêm suas próprias condições, conforme a [política de licenciamento](LICENCIAMENTO_E_ECOSSISTEMA.md).

O build Windows MSVC usa `crt-static`, conforme a [referência de Rust](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes). O executável foi inspecionado sem imports de `VCRUNTIME140.dll`/MSVCP; bibliotecas nativas do Windows permanecem. O verificador da CI impede regressão dessa dependência. Builds por script entram na raiz para aplicar `.cargo/config.toml` e o toolchain fixado mesmo quando chamados de outro diretório.

O pacote da engine não inclui Ollama/modelos ou SDKs dos aplicativos. Instalar a engine dispensa Rust e Python; gerar um app continua exigindo o provider e o SDK de seu [perfil](PERFIS_V1.md). O script Windows instala em `releases/v1`, associa `.crexe` e registra **Gerar com CREXE** para arquivos comuns, sem alterar o aplicativo padrão desses documentos. O pacote também inclui `unassociate.cmd`, que remove os registros pertencentes à instalação. Linux/macOS têm instalação por CLI e entrada gráfica pelo seletor na janela da engine, sem associação `.crexe` ou menu de gerenciador implementado.

Os instaladores interativos abrem a configuração nativa ao concluir. O pacote inclui `configure.cmd` no Windows, `configure.command` no macOS ou `configure.sh` no Linux. Eles usam a cópia instalada ou, se ausente, a cópia extraída. `crexe install` continua não interativo, inclusive sem display. A GUI usa egui/eframe com OpenGL; macOS/Linux precisam de sessão gráfica e bibliotecas de desktop disponíveis. Salvar chaves usa o cofre nativo (Secret Service no Linux). Verificar requisitos reais em máquinas limpas permanece parte da aceitação; build em CI não comprova todas as sessões gráficas.

O checkpoint `845e1bc` produziu e verificou os três pacotes na [CI 36096064360](https://github.com/omarceloribeiro/crexe/actions/runs/36096064360): Windows/x86_64, Linux/x86_64 e macOS/aarch64. Os arquivos baixados tiveram checksum e todos os hashes internos conferidos neste host. O binário Windows produzido no GitHub foi instalado em uma raiz de teste e executou `--version`, `doctor` e `inspect` com PATH vazio, a partir de outra pasta. [Registro dos pacotes](validation/2026-09-25-engine-packages.json).

Esse checkpoint é anterior a **Gerar com CREXE**. Seus hashes e ensaios não validam os novos pacotes; a prévia com a feature deve ser gerada da revisão correspondente e receber novos checksums e evidências.

## Gerar com CREXE no pacote

O pacote inclui `examples/primeira-leitura.md` e `examples/outra-leitura.md`, com títulos diferentes. Na pasta extraída, execute:

```powershell
.\crexe.exe open-generating -- examples/primeira-leitura.md
.\crexe.exe open-generating -- examples/outra-leitura.md
```

No macOS/Linux, substitua `.\crexe.exe` por `./crexe`. Na primeira preparação do tipo, há uma chamada à LLM para formular a intenção do leitor a partir somente da extensão, seguida da geração normal de código e das correções permitidas. O planejamento não consome uma tentativa de código; o padrão permite um plano e até três tentativas de código, sujeitos aos limites de tokens e tempo. O segundo arquivo deve usar o leitor compatível aprovado, sem outra chamada ou compilação. O programa recebe o caminho em runtime e abre o documento escolhido; nome, caminho e conteúdo não integram o pedido do leitor ou os fontes exportados. Extensões desconhecidas válidas são aceitas. Arquivos sem extensão, com extensão inválida ou `.crexe` recebem aviso sem chamada ao modelo; receitas usam a abertura normal. O contrato inicial é somente leitura, com texto ou inspeção hexadecimal para formatos opacos, sem promessa de isolamento de segurança.

Na janela da engine, **Selecionar arquivo e gerar programa…** oferece o mesmo fluxo. A configuração ativa salva é usada; a seleção não salva alterações pendentes nos campos. No Windows, **Gerar com CREXE** aparece no menu de um arquivo após a associação; no Windows 11, pode ficar em **Mostrar mais opções**. O menu só atende seleção única. Finder/GNOME/Dolphin usam o seletor da janela como alternativa nesta prévia.

A CI inclui fixtures de leitor com provider local controlado e .NET 8 nos três hosts, sem modelo real ou display, além da suíte Rust e dos perfis nativos existentes. Um job separado verifica o site PT/EN, Markdown, links e downloads. Esses checks não comprovam presença no menu real, o seletor em cada sessão gráfica nem a qualidade de geração de um modelo real. Registre esses ensaios por plataforma antes de anunciá-los como verificados.

A instalação não altera o PATH. No PowerShell, use `.\crexe.exe` dentro da pasta extraída ou o caminho completo da instalação. Para concluir os ensaios fora do Codex, siga o [roteiro Windows](VALIDACAO_INSTALACAO_WINDOWS.md).

Antes de uma release pública, ainda é necessário validar instalação/atualização fora do ambiente do Codex, primeira execução em máquina limpa, requisitos mínimos de OS/arquitetura, políticas de assinatura/distribuição e os cenários de app declarados estáveis. O ambiente do Codex pode virtualizar caminhos AppData; testes feitos nele não comprovam a instalação percebida pelo Explorer em uma sessão normal.
