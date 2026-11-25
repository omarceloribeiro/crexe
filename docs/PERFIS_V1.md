# Perfis de intenção e pré-requisitos

Todos compilam no host do usuário, em pasta temporária. A engine não instala SDKs, não emula outro OS e não usa Windows Sandbox. Os perfis nativos novos são **experimentais**, até completarem os ensaios de geração real e uso da GUI previstos no plano.

| Perfil | Host | SDK/ferramentas | Saída |
|---|---|---|---|
| `dotnet-winforms` | Windows | .NET 8 SDK no PATH | C#, Windows Forms, projeto fornecido pela engine |
| `dotnet-console` | Windows/Linux/macOS | .NET 8 SDK no PATH | C#, console; escolha explícita |
| `cpp-win32` | Windows | MinGW-w64 `g++` no PATH, target compatível com a engine | Vários `.cpp/.h`, Win32 Unicode, executável com runtime GCC ligado estaticamente |
| `objc-cocoa` | macOS | Apple Clang/Xcode Command Line Tools e SDK Cocoa | Vários `.mm/.h`, ARC, executável nativo |
| `c-gtk` | Linux | `gcc`, GNU Make, `pkg-config` e desenvolvimento GTK3 | Vários `.c/.h`, Makefile fornecido pela engine |
| `dotnet-opengl` | Windows | .NET 8 SDK e driver OpenGL 3.3 | C#, OpenTK 4.9.4, projeto e lockfile NuGet |
| `objc-cocoa-opengl` | macOS | Apple Clang, Cocoa e framework OpenGL; contexto 3.2 core | Objective-C++, `NSOpenGLView` |
| `c-gtk-opengl` | Linux | GCC, Make, GTK3 e libepoxy via `pkg-config`; contexto 3.3 | C, `GtkGLArea`, Makefile da engine |

Na seleção automática, o modelo recebe o catálogo dos perfis que passaram pela verificação de SDKs e política local. A preferência inicial é Windows Forms/.NET no Windows, GTK3 no Linux e Cocoa no macOS; nas revisões seguintes, prefere o perfil do baseline. Pode mudar de tecnologia com justificativa na mesma resposta de geração, sem uma chamada adicional de planejamento. “Botões 3D” não é uma regra fixa de seleção. `--profile nome`, o metadado Markdown `profile` e targets de YAML fixam a tecnologia e impedem mudanças pelo modelo.

Antes de chamar o modelo, .NET exige um SDK 8 instalado e os perfis nativos conferem o target do compilador. GTK confere também `pkg-config --exists gtk+-3.0`. Falhas de link/build continuam possíveis, com diagnóstico. Essa verificação básica não detecta todos os SDKs de terceiros, drivers ou requisitos do prompt.

`dotnet-opengl` usa [`OpenTK` 4.9.4](https://www.nuget.org/packages/OpenTK/4.9.4), com versão exata no projeto, `dotnet restore --use-lock-file` e restaurações seguintes em modo bloqueado. O lockfile aprovado acompanha incrementos e ZIPs. A engine restaura bibliotecas por projeto; não instala SDKs/compiladores. GTK3/libepoxy e frameworks Apple são pré-requisitos do host, sem download ou lockfile de pacotes do sistema pela engine.

No Linux, `pkg-config` verifica também libepoxy; o contrato usa [`GtkGLArea`](https://docs.gtk.org/gtk3/class.GLArea.html) e exige diagnóstico de erro ao criar o contexto. No macOS, [`NSOpenGLView`](https://developer.apple.com/documentation/appkit/nsopenglview?language=objc) e OpenGL estão depreciados. Os contratos OpenGL exigem verificar o contexto/driver na abertura e informar incompatibilidade, além de oferecer `--crexe-graphics-check` para criar contexto, compilar shaders e renderizar um quadro. Autotestes de lógica continuam sem display; aprovação do build não comprova suporte gráfico no destino.

Ao gerar para abrir o programa, a engine executa também `--crexe-graphics-check` antes de aprovar a revisão; na reabertura pelo cache, repete a verificação antes de iniciar a interface. `--no-run` mantém o fluxo sem display, executando os testes de lógica. O teste gráfico tem timeout e diagnóstico próprios de teste; sua implementação é parte dos fontes gerados, assim como o autoteste de lógica.

`crexe doctor` mostra perfil, linguagem, requisito de SDK e caminhos das ferramentas, sem executá-las. `crexe doctor --check` executa as mesmas sondagens locais do início da geração; `--profile nome` seleciona outro perfil. As consultas respeitam a política local de ferramentas, têm timeout de 15 s cada e não chamam providers, leem chaves, instalam ferramentas ou compilam um aplicativo. O comando retorna falha quando falta o SDK ou a política impede a consulta. Encontrar um caminho no `doctor` simples não significa que o SDK foi validado.

`doctor` e `inspect` distinguem a arquitetura da engine da informada pelo OS. Windows consulta [IsWow64Process2](https://learn.microsoft.com/en-us/windows/win32/api/wow64apiset/nf-wow64apiset-iswow64process2), carregada dinamicamente; macOS consulta [sysctl.proc_translated](https://developer.apple.com/documentation/apple-silicon/about-the-rosetta-translation-environment) para reconhecer Rosetta e usa `uname` no processo nativo; Linux usa `uname`. Em falha de detecção o host aparece como desconhecido. A observação descreve o ambiente visível ao OS, não prova o hardware físico fora de uma VM ou contêiner. O build e `ARCH` continuam usando a arquitetura da engine; detecção diferente não ativa cross-compilation. Cenários reais de emulação ainda precisam de validação.

Os comandos de compilação pertencem à engine. C++/Cocoa recebem a lista explícita dos fontes gerados, sem limitar o projeto a um arquivo. GTK recebe um Makefile que resolve os flags na máquina que compila, usando o fluxo de [compilação documentado pelo GTK](https://docs.gtk.org/gtk3/compiling.html). Headers e recursos podem acompanhar os fontes; esses perfis não incluem um gerenciador geral de dependências externas.

Todos pedem `--crexe-self-test` antes de iniciar a UI e executam esse teste antes de aceitar o cache. Esse teste é produzido pelo modelo e não substitui testes independentes. `--no-run` ainda compila e testa. A revisão aprovada registra o perfil e comandos efetivos; cache, reparos e exportação usam esse perfil. Respostas antigas com apenas `files` mantêm o perfil corrente.

O ZIP inclui fontes, manifesto, scripts e arquivos de build. Em C++/Cocoa/GTK, publish recompila um executável `CrexeApp-publish`; não cria instalador, pacote de bibliotecas GTK, assinatura/notarização ou bundle `.app`. No .NET, publish depende do runtime. Bibliotecas do sistema e SDKs necessários devem estar presentes no destino; não há promessa de binário universal ou distribuição pronta entre quaisquer máquinas.

## Verificação automatizada

`cargo test --locked` testa a engine sem SDKs de apps adicionais. O ensaio nativo é separado:

```sh
cargo test --locked --test native_profiles -- --ignored --nocapture
```

Ele recebe dois fontes fixos de um servidor Ollama simulado local, compila, executa o teste de lógica sem display, exporta/recompila o ZIP pelos scripts e publica em outra pasta. O cache também é aberto com PATH vazio e o servidor encerrado. Não usa um modelo real nem interage com janelas.

A CI prepara o SDK somente nos runners descartáveis: [MSYS2 UCRT64](https://github.com/msys2/setup-msys2) no Windows, pacotes GTK3 no Ubuntu e SDK Apple existente no macOS. Esses passos da CI não são instaladores para o usuário final. Os resultados e limitações ficam no [registro de implementação](IMPLEMENTACAO_V1.md).

O ensaio de migração OpenGL usa fontes controlados e verifica continuidade de contexto, mudança de perfil, incremento posterior, cache, lockfile quando aplicável, exportação e recompilação:

```sh
cargo test --locked --test opengl_profiles -- --ignored --nocapture
```

Defina `CREXE_TEST_GRAPHICS=1` em uma sessão gráfica com driver compatível para executar também a renderização. Sem essa variável, o teste compila os fontes OpenGL e executa somente as verificações de lógica. A CI inclui esse ensaio nos três hosts; inclusão no workflow não significa que uma execução remota deste incremento já foi aprovada.
