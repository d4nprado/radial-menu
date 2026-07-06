# Orbit Launcher

Menu radial desktop construído com Tauri v2, Vue 3 e TypeScript. Por padrão,
pressione `Ctrl+Space` para abrir o launcher na posição atual do cursor.

## Desenvolvimento

Pré-requisitos no Windows:

- Node.js 20 ou mais recente;
- Rust stable;
- Microsoft C++ Build Tools;
- WebView2 Runtime (já incluído no Windows 11).

Instale e execute:

```powershell
npm install
npm run tauri dev
```

Para validar o frontend:

```powershell
npm run build
```

Para gerar um instalador:

```powershell
npm run tauri build
```

## Configuração

Os itens são configurados pela janela do launcher e persistidos em
`launcher-config.json`, no diretório de dados do aplicativo. Uma instalação
nova começa com o menu vazio. O atalho global pode ser alterado nas
preferências e é salvo separadamente em `app-preferences.json`.

## Organização

- `src/components`: apresentação do menu e dos itens;
- `src/composables/useMenuActions.ts`: registro de executores de ações;
- `src/composables/useSystemStats.ts`: atualização do status enquanto o menu está aberto;
- `src/types/menu.ts`: contratos das ações e da configuração;
- `src-tauri/src/commands.rs`: fronteira nativa com validação dos caminhos;
- `src-tauri/src/system_stats.rs`: coleta assíncrona de CPU, RAM, disco e rede;
- `src-tauri/src/lib.rs`: inicialização e registro do atalho global.
