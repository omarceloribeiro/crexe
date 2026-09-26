# Validar a instalação Windows fora do Codex

Este roteiro cobre os ensaios manuais ainda pendentes. Os testes automatizados executados dentro do Codex podem enxergar AppData virtualizado; eles não comprovam o percurso do Explorer em uma sessão normal. Use um pacote de desenvolvimento, sem tratar o resultado como release estável.

## Preparação

1. Baixe o artefato Windows de uma execução aprovada da CI em `feature/v1`. Extraia o ZIP externo do GitHub para encontrar o ZIP da engine e seu arquivo `.sha256`.
2. Em um PowerShell aberto pelo menu Iniciar, execute `Get-FileHash caminho-do-pacote.zip -Algorithm SHA256` e compare com o `.sha256`. Extraia o pacote da engine para uma pasta nova. Não mova o cache do ambiente de desenvolvimento.
3. Na pasta que contém `crexe.exe`, execute `.\crexe.exe doctor`. A engine deve iniciar mesmo sem Rust/Python no PATH. Registre versão, arquitetura e caminho de instalação mostrado.
4. Execute `.\crexe.exe doctor --check --profile dotnet-winforms`. O SDK .NET 8 deve estar disponível. Sem ele, o resultado esperado é uma falha explicando o requisito, sem chamar a IA. Prepare o provider conforme [configuração](CONFIGURACAO.md); o default é Ollama local com `gemma4:12b`.

## Instalação e associação

Execute `.\install.cmd` nesse terminal. O destino esperado é `%LOCALAPPDATA%\CREXE\releases\v1\crexe.exe`, na conta do usuário comum, sem precisar de administrador. O script instala, registra a associação e abre Configurações; não acrescenta a pasta ao PATH. Configure o provider/modelo e salve. Reabra pelo `configure.cmd`. Se tiver editado o exemplo do pacote, importe esse TOML na tela, revise e salve; reinstalar não importa o exemplo automaticamente.

Confirme a cópia instalada:

```powershell
& "$env:LOCALAPPDATA\CREXE\releases\v1\crexe.exe" doctor --check
```

Copie apenas `examples\calculadora.crexe` para outra pasta, fora do pacote e do checkout. No Explorer, use dois cliques. Se o Windows pedir um aplicativo, escolha a engine nesse caminho estável e marque a opção de usá-la para `.crexe`.

## Percurso de aceitação

| Ação | Resultado a conferir |
|---|---|
| Abrir a intenção pela primeira vez | Janela Creative Executable aparece, mostra a preparação e fecha quando a janela do aplicativo está pronta (espera máxima de 15 s na etapa de abertura). O app deve estar restaurado; conferir foco separadamente. |
| Clicar várias vezes durante geração/build/abertura | Uma única preparação; a janela existente volta à frente quando permitido. Sem novas gerações em fila. |
| Editar durante uma preparação e reabrir | Orientação para concluir/cancelar a preparação existente antes de reabrir. |
| Reabrir enquanto o aplicativo já está pronto | Outra instância abre usando cache. |
| Salvar local → OpenAI e reabrir uma nova intenção | Usa somente o provider selecionado; a chave pode ser configurada pela tela. Trocar de volta deve usar Ollama. |
| Editar TOML externamente com a tela aberta e tentar salvar | Detecta conflito; Recarregar permite revisar antes de salvar. |
| Fechar Configurações sem salvar | Nenhuma preferência é alterada. Reinstalar também preserva o que estava salvo. |
| Usar a calculadora | Quatro operações, limpar e divisão por zero apresentam comportamento adequado; anote qualquer divergência do prompt. |
| Fechar e abrir novamente | Mesmo aplicativo reaparece por cache, sem nova geração. |
| Editar a intenção pedindo outra cor, salvar e abrir | Uma nova geração ocorre e o aplicativo reflete a alteração. |
| Abrir novamente sem editar | Nova versão usa o cache. |
| Alterar outra vez e fechar a janela de espera durante geração | Preparação cancela e não abre aplicativo incompleto. Restaurar os bytes da intenção anterior permite reutilizar seu cache. |
| Instalar outro pacote 1.x e repetir a abertura | Associação continua apontando para `releases/v1`, sem precisar refazê-la para um novo diretório de versão. |
| Copiar somente o `.crexe` para outro Windows preparado | O segundo host gera e compila sua própria revisão; nenhum cache foi transportado. |

As gerações reais usam os recursos do provider configurado. Cancelamento não garante que um servidor remoto deixou de processar uma requisição já enviada; a engine não compra créditos nem troca para OpenAI automaticamente.

Para verificar uma falha visível sem chamar a IA, salve `erro.crexe` com o conteúdo abaixo e abra por dois cliques. Deve aparecer uma mensagem de perfil desconhecido; nenhum programa deve ser gerado:

```markdown
---
crexe: 1
profile: perfil-inexistente
---
Mostrar uma saudação.
```

## Registro

Anote commit do `PACKAGE.json`, SHA-256 do ZIP, versão/arquitetura do Windows, SDK/perfil/provider/modelo e resultados de cada linha. Distinga instalação fora do Codex, máquina limpa, testes por CLI e cliques manuais. Não inclua chaves ou configurações com segredos no relato.

Para remoção da associação, use `crexe.exe unassociate` pelo caminho completo instalado. O comando deve preservar registros de outros aplicativos. A escolha efetiva do padrão pelo Windows pode exigir ajuste em Configurações ou “Abrir com”.
