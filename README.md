# Orbit Launcher

Menu radial desktop construído com Tauri v2, Vue 3 e TypeScript. Pressione
`Ctrl+Space` para abrir o launcher na posição atual do cursor.

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

Os seis itens ficam em `src/config/menu.json`. Cada item aceita uma ação
`program` ou `directory`, sempre com um caminho absoluto. O campo `accent`
controla a cor do item e `icon` aceita uma letra ou símbolo curto.

O atalho efetivamente registrado fica na constante `GLOBAL_SHORTCUT`, em
`src-tauri/src/lib.rs`. Atualize também `shortcut` no JSON para manter o texto
mostrado no centro do menu sincronizado.

## Organização

- `src/components`: apresentação do menu e dos itens;
- `src/composables/useMenuActions.ts`: registro de executores de ações;
- `src/composables/useSystemStats.ts`: atualização do status enquanto o menu está aberto;
- `src/types/menu.ts`: contratos das ações e da configuração;
- `src-tauri/src/commands.rs`: fronteira nativa com validação dos caminhos;
- `src-tauri/src/system_stats.rs`: coleta assíncrona de CPU, RAM, disco e rede;
- `src-tauri/src/lib.rs`: inicialização e registro do atalho global.

Para adicionar OBS futuramente, crie um novo tipo em `MenuAction` e registre seu
executor no composable, mantendo o transporte WebSocket em um módulo dedicado.
Mouse 4/5 poderá entrar como outra fonte de acionamento no backend e chamar a
mesma função `show_window_at_cursor`; a lógica do menu não depende do teclado.
