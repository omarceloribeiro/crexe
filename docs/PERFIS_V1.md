# Perfis de intenção e pré-requisitos

Todos compilam no host do usuário, em pasta temporária. A engine não instala SDKs, não emula outro OS e não usa Windows Sandbox. Os perfis nativos novos são **experimentais**, até completarem os ensaios de geração real e uso da GUI previstos no plano.

| Perfil | Host | SDK/ferramentas | Saída |
|---|---|---|---|
| `dotnet-winforms` | Windows | .NET 8 SDK no PATH | C#, Windows Forms, projeto fornecido pela engine |
| `dotnet-console` | Windows/Linux/macOS | .NET 8 SDK no PATH | C#, console; escolha explícita |
| `cpp-win32` | Windows | MinGW-w64 `g++` no PATH, target compatível com a engine | Vários `.cpp/.h`, Win32 Unicode, executável com runtime GCC ligado estaticamente |
| `objc-cocoa` | macOS | Apple Clang/Xcode Command Line Tools e SDK Cocoa | Vários `.mm/.h`, ARC, executável nativo |
| `c-gtk` | Linux | `gcc`, GNU Make, `pkg-config` e desenvolvimento GTK3 | Vários `.c/.h`, Makefile fornecido pela engine |

Na seleção automática, Windows prefere .NET se `dotnet` existe no PATH; usa C++ quando há `g++` e não há `dotnet`. Linux usa GTK3 e macOS Cocoa. Uma ferramenta encontrada ainda precisa passar pela verificação prévia; a engine não troca silenciosamente de perfil após uma incompatibilidade. `--profile nome` ou o metadado Markdown `profile` permitem a escolha explícita.

Antes de chamar o modelo, .NET exige um SDK 8 instalado e os perfis nativos conferem o target do compilador. GTK confere também `pkg-config --exists gtk+-3.0`. Falhas de link/build continuam possíveis, com diagnóstico. Essa verificação básica não detecta todos os SDKs de terceiros, drivers ou requisitos do prompt.

`crexe doctor` mostra perfil, linguagem, requisito de SDK e caminhos das ferramentas, sem executá-las. `crexe doctor --check` executa as mesmas sondagens locais do início da geração; `--profile nome` seleciona outro perfil. As consultas respeitam a política local de ferramentas, têm timeout de 15 s cada e não chamam providers, leem chaves, instalam ferramentas ou compilam um aplicativo. O comando retorna falha quando falta o SDK ou a política impede a consulta. Encontrar um caminho no `doctor` simples não significa que o SDK foi validado.

`doctor` e `inspect` distinguem a arquitetura da engine da informada pelo OS. Windows consulta [IsWow64Process2](https://learn.microsoft.com/en-us/windows/win32/api/wow64apiset/nf-wow64apiset-iswow64process2), carregada dinamicamente; macOS consulta [sysctl.proc_translated](https://developer.apple.com/documentation/apple-silicon/about-the-rosetta-translation-environment) para reconhecer Rosetta e usa `uname` no processo nativo; Linux usa `uname`. Em falha de detecção o host aparece como desconhecido. A observação descreve o ambiente visível ao OS, não prova o hardware físico fora de uma VM ou contêiner. O build e `ARCH` continuam usando a arquitetura da engine; detecção diferente não ativa cross-compilation. Cenários reais de emulação ainda precisam de validação.

Os comandos de compilação pertencem à engine. C++/Cocoa recebem a lista explícita dos fontes gerados, sem limitar o projeto a um arquivo. GTK recebe um Makefile que resolve os flags na máquina que compila, usando o fluxo de [compilação documentado pelo GTK](https://docs.gtk.org/gtk3/compiling.html). Headers e recursos podem acompanhar os fontes; esses perfis não incluem um gerenciador geral de dependências externas.

Todos pedem `--crexe-self-test` antes de iniciar a UI e executam esse teste antes de aceitar o cache. Esse teste é produzido pelo modelo e não substitui testes independentes. `--no-run` ainda compila e testa. Cache válido com perfil explícito dispensa o compilador/provider; seleção automática pode mudar se o inventário do PATH mudar.

O ZIP inclui fontes, manifesto, scripts e arquivos de build. Em C++/Cocoa/GTK, publish recompila um executável `CrexeApp-publish`; não cria instalador, pacote de bibliotecas GTK, assinatura/notarização ou bundle `.app`. No .NET, publish depende do runtime. Bibliotecas do sistema e SDKs necessários devem estar presentes no destino; não há promessa de binário universal ou distribuição pronta entre quaisquer máquinas.

## Verificação automatizada

`cargo test --locked` testa a engine sem SDKs de apps adicionais. O ensaio nativo é separado:

```sh
cargo test --locked --test native_profiles -- --ignored --nocapture
```

Ele recebe dois fontes fixos de um servidor Ollama simulado local, compila, executa o teste de lógica sem display, exporta/recompila o ZIP pelos scripts e publica em outra pasta. O cache também é aberto com PATH vazio e o servidor encerrado. Não usa um modelo real nem interage com janelas.

A CI prepara o SDK somente nos runners descartáveis: [MSYS2 UCRT64](https://github.com/msys2/setup-msys2) no Windows, pacotes GTK3 no Ubuntu e SDK Apple existente no macOS. Esses passos da CI não são instaladores para o usuário final. Os resultados e limitações ficam no [registro de implementação](IMPLEMENTACAO_V1.md).
