# Limites de execução

CREXE v1 em desenvolvimento executa fontes gerados e ferramentas de build com as permissões do usuário, numa pasta temporária no host. Isso **não oferece isolamento de segurança**. Um programa malicioso pode acessar arquivos, rede e recursos do usuário.

Validação de caminhos da engine, hashes de cache, destinos locais de credenciais, limites de resposta/logs e grupos de subprocessos reduzem falhas específicas. Não tornam o host seguro para código não confiável. O modelo não é autoridade de permissões.

Provider, endpoint e variável de credencial são escolhidos localmente; `.env` só é lido explicitamente. Segredos conhecidos não são herdados por subprocessos. Um programa nativo ainda pode procurar outros arquivos/configurações do usuário; a engine não promete impedir isso.

Windows Sandbox é proibido nesta fase. Docker/WSL não são dependências nem backends. Requisitos de isolamento não implementados devem gerar erro, nunca uma indicação falsa de proteção.

Relate problemas com versão e exemplo mínimo com dados sintéticos. Não publique credenciais, dados pessoais, caches privados ou dumps do ambiente em issues. Rotacione credenciais expostas.
