# Operações de projeto da v1

`exec` transforma uma intenção em projeto validado. As operações abaixo recebem uma **pasta com CREXE-PROJECT.json**, como uma revisão ou ZIP extraído. Não recebem um `.crexe` nem iniciam geração/reparo por IA.

| Comando | Resultado |
|---|---|
| `crexe build pasta --output nova-pasta` | Copia os fontes para um workspace temporário, compila, executa os testes definidos e entrega projeto/artefatos em nova pasta. |
| `crexe test pasta` | Recompila e executa os testes definidos; falha se não houver operação test. |
| `crexe run pasta` | Executa o artefato existente declarado no manifesto; não compila nem escolhe outro executável. |
| `crexe publish pasta --output nova-pasta` | Compila, testa quando definido e executa publish, se existir. Produz arquivos locais; não faz deploy. |
| `crexe export pasta --output novo.zip` | Empacota os fontes atuais, manifesto e scripts, sem build/teste. Não afirma que edições posteriores foram validadas. |

Destinos existentes são recusados. Build/test/publish preservam a origem, inclusive em falha. O workspace de diagnóstico é mantido quando uma operação falha. Editar fontes deve ser feito no ZIP extraído ou em uma cópia de trabalho, preservando a revisão de cache.

O manifesto versão 1 registra nome, OS, arquitetura da engine, perfil, lista de fontes e vetores de argumentos para cada operação. Build/run são obrigatórios; test/publish opcionais. A engine gera scripts BAT ou SH a partir dessas mesmas listas. Metadados e scripts reservados são escritos pela engine, não substituídos pela resposta do modelo.

Operações de execução verificam o OS e a arquitetura registrados; não fazem cross-compilation implícita. Export pode ser usado para examinar/compartilhar fontes em outro OS. O manifesto é dado local revisável, não uma assinatura de confiança. Alterar comandos não dispensa a política local de ferramentas da CLI.

No perfil .NET, publish usa `dotnet publish --self-contained false`: o destinatário ainda precisa do runtime compatível. O ZIP inclui fontes e instruções, sem SDK, cache de pacotes ou credenciais. Usar os scripts sem CREXE exige preparar o SDK indicado; eles executam diretamente no host e não aplicam a política TOML da engine.

Manifestos antigos ausentes não são inferidos a partir de executáveis encontrados. Os scripts de um ZIP anterior continuam disponíveis; para as novas operações, gere um projeto com a engine atual.
