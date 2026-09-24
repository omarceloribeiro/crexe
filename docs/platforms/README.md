# Uma base Rust para Windows, Linux e macOS

Status: orientação de implementação da v1. A engine original continua em `src/`; os módulos abaixo ainda serão extraídos conforme o [plano](../PLANO_V1.md). O baseline fixo `baseline/pre-v1` permanece inalterado.

**Regra atual do autor: não usar Windows Sandbox em hipótese alguma nesta fase.** Não instalar, habilitar, iniciar, testar ou invocar esse produto, inclusive por arquivos `.wsb`, `WindowsSandbox.exe`, comandos gerados ou fallback. Uma possível feature futura depende de nova decisão. A execução nativa no Windows e outros mecanismos de isolamento de processos continuam sendo opções a avaliar conforme o plano.

## Mecanismo padrão

Usar a compilação condicional nativa de Rust, `#[cfg(...)]`, e as dependências por target do Cargo. O compilador seleciona os módulos adequados ao sistema de destino. Não remover arquivos, comentar imports manualmente nem manter uma cópia inteira da engine por sistema operacional. Esse mecanismo é documentado na [referência oficial de Rust](https://doc.rust-lang.org/reference/conditional-compilation.html).

O núcleo compartilhado contém parsers, configuração, providers, plano de projeto, cache e orquestração. Código específico de janela, console, associação de arquivos, caminhos do sistema e sandbox fica nos módulos responsáveis por essas integrações. Os tipos públicos e eventos consumidos pelo núcleo não devem expor tipos exclusivos de uma API Windows.

## Organização da apresentação

Estrutura proposta após a reorganização do pacote:

```text
src/
  progress.rs                 # eventos de fase, falha e início do app
  pipeline.rs                 # execução compartilhada
  presentation/
    mod.rs                    # interface e seleção da apresentação
    console.rs                # mensagens de terminal
    windows.rs                # janela/integração específica de Windows
    linux.rs                  # implementação Linux, quando disponível
    macos.rs                  # implementação macOS, quando disponível
```

Exemplo ilustrativo de seleção em `presentation/mod.rs`, quando os respectivos arquivos existirem:

```rust
mod console;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod macos;
```

Assim, `windows.rs` não participa de uma compilação destinada ao Linux. `target_os` identifica o destino do binário da engine; a escolha do target de um aplicativo gerado continua sendo responsabilidade do plano de execução.

Criar implementações conforme forem entregues, sem arquivos vazios para completar a árvore. Enquanto uma plataforma não tiver janela, manter a CLI funcional e responder claramente quando o usuário solicitar `--ui`. O pipeline emite os mesmos eventos para qualquer apresentação, sem duplicar geração, compilação, cache ou reparo. Se uma biblioteca de UI multiplataforma for adotada, compartilhar também seu código de apresentação e isolar apenas as integrações que realmente forem específicas.

Aplicar `#[cfg]` tanto aos módulos quanto aos imports/chamadas que dependem deles. Um `if cfg!(...)` não elimina os ramos do código da verificação do compilador; ele não substitui o atributo para esconder APIs de outro sistema. Ver [Rust by Example](https://doc.rust-lang.org/rust-by-example/attribute/cfg.html).

## Dependências e execução sem desktop

Bibliotecas usadas exclusivamente no Windows pertencem à seção `[target.'cfg(target_os = "windows")'.dependencies]` do `Cargo.toml`; o mesmo princípio vale para Linux/macOS. Usar crates mantidas quando necessárias, sem inventar um sistema de portabilidade ou seleção de arquivos. Versões e recursos concretos serão definidos na implementação e registrados no lockfile. A [documentação do Cargo](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies) define essa sintaxe.

Quando a biblioteca de janela introduzir dependências gráficas, permitir uma compilação só de CLI por meio das features padrão do Cargo e dependências opcionais. Não usar features manuais como `windows`/`linux` para substituir a detecção do target. O modo sem desktop deve funcionar sem exigir bibliotecas gráficas apenas para ler um `.crexe`, gerar ou compilar um projeto. A opção de runtime `--ui` não torna opcional uma dependência que já foi vinculada ao executável; essa separação deve existir também no build. Ver [features do Cargo](https://doc.rust-lang.org/cargo/reference/features.html).

A implementação de Windows também deve tratar as diferenças entre abertura pelo Explorer e uso pelo terminal. Esses ajustes não devem alterar o entrypoint de Linux/macOS. Janela responsiva, cancelamento, erros e fechamento ao iniciar o app seguem o contrato do plano.

## Ambiente dos programas gerados

A seleção de módulos Rust decide como compilar a engine. A seleção do ambiente de build/teste/run de um programa gerado é outra decisão, feita em runtime por requisitos e política local, conforme a seção “Escolha entre host nativo e sandbox Linux” do plano.

- Um perfil Windows que exige ferramentas ou drivers locais usa um backend Windows compatível; a presença de Docker Linux não muda o target solicitado.
- Um projeto ou fase compatível com Linux pode usar o contêiner disponível e preparado, quando permitido.
- O modo host deve estar previsto na configuração local e informa seu nível real de isolamento. Não escolhê-lo silenciosamente depois de uma falha no sandbox.
- Dependências são verificadas dentro do ambiente em que serão usadas. Modelos podem sugerir requisitos, mas não concedem permissões nem escolhem comandos arbitrários de descoberta.
- Build, testes com mocks e testes físicos podem usar ambientes distintos. Os resultados devem dizer o que foi efetivamente validado.

Para reduzir o tempo até abrir o app, preferir um único ambiente compatível e não repetir build/testes em outro backend sem necessidade. Quando a intenção exigir interpretação, uma triagem curta por IA identifica requisitos de SO, APIs, ferramentas e dispositivos por fase; essa resposta é validada pela engine e não concede permissões. A pergunta deve abranger também macOS/Linux, sem classificar tudo como “Windows ou não Windows”. Perfis já resolvidos e cache válido dispensam a chamada. Se já houver planejamento inicial por IA, incorporar a triagem nessa resposta, sem duplicação. Medir a duração e o carregamento do modelo; não prometer que toda chamada curta terá baixa latência.

Não usar Windows Sandbox para resolver incompatibilidades ou requisitos ausentes. Casos sem backend permitido recebem diagnóstico. Exemplos como Unity, biometria e impressão orientam a arquitetura; suporte concreto depende de perfis e validações próprios, sem ampliar implicitamente a matriz da v1.

## Compilação e verificação

Na organização atual, o comando de build do pacote a partir da raiz é:

```sh
cargo build --release --manifest-path src/Cargo.toml
```

Após mover o manifest para a raiz e versionar `Cargo.lock`, o comando previsto é:

```sh
cargo build --release --locked
```

Executar o build em cada sistema com Rust e as ferramentas nativas necessárias. O mesmo checkout deve servir para todos, sem remover especializações. O Cargo permite selecionar outro destino com `--target`, mas a disponibilidade de linker, SDKs e bibliotecas para esse destino precisa ser preparada e verificada. Usar builds nativos em CI como percurso inicial; não prometer que uma única máquina já produz e testa os três executáveis. Referência: [`cargo build`](https://doc.rust-lang.org/cargo/commands/cargo-build.html).

Critérios do CREXE:

- Compilar e testar o núcleo compartilhado nos sistemas anunciados, a partir do mesmo commit.
- Confirmar que uma compilação Linux/macOS não tenta compilar imports de Windows.
- Validar separadamente os modos CLI sem desktop e GUI disponíveis; compilar o núcleo não comprova funcionamento da janela.
- Manter as diferenças de instalação, associação e sandbox atrás de módulos específicos, usando o mesmo mecanismo padrão.
- Registrar no README público a matriz de suporte, requisitos e comandos efetivamente verificados. A janela Windows pode ser entregue primeiro sem impedir os builds da CLI nos demais sistemas.
