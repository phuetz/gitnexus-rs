# GitNexus launcher scripts

Scripts Windows pour lancer GitNexus sans retenir les longues commandes Cargo/npm.

## Lancer le client chat React

Depuis la racine du projet :

```powershell
.\start-chat-react.cmd
```

Ce lanceur :

- demarre le backend HTTP GitNexus sur `http://127.0.0.1:3010`,
- configure `chat-ui/.env.local` avec `VITE_MCP_URL=http://127.0.0.1:3010`,
- demarre le client React sur `http://127.0.0.1:5176`,
- ouvre le navigateur.

Si le backend ou le client React repond deja sur le port demande, le script le
reutilise au lieu d'ouvrir une deuxieme instance. Si le port est occupe par un
service qui ne repond pas comme GitNexus, le script s'arrete avec un message
clair au lieu de basculer silencieusement vers un autre port.
Le lanceur nettoie aussi les anciens processus Vite GitNexus du meme `chat-ui`
et du meme port lorsqu'ils ecoutent sur un autre host, afin d'eviter les
conflits `localhost` / `127.0.0.1`.

Variante sans backend si le serveur tourne deja :

```powershell
.\gitnexus.cmd chat -NoBackend
```

Changer les ports :

```powershell
.\gitnexus.cmd chat -BackendPort 3001 -ChatPort 5175
```

Compatibilite avec l'ancien onglet `localhost:5174` :

```powershell
.\gitnexus.cmd chat -ChatPort 5174
```

Redemarrer explicitement apres une modification backend ou UI :

```powershell
.\gitnexus.cmd chat -RestartBackend
.\gitnexus.cmd chat -RestartChat
.\gitnexus.cmd chat -RestartBackend -RestartChat
```

`-RestartBackend` n'arrete que le processus `gitnexus` qui ecoute sur le port
backend choisi. Si le port est occupe par une autre application, le script
refuse de l'arreter.

## Lancer l'application desktop

```powershell
.\start-desktop.cmd
```

Ce lanceur demarre l'UI Vite desktop sur `http://localhost:1421`, puis lance Tauri.

## ChatGPT OAuth

Configurer GitNexus pour utiliser l'abonnement ChatGPT avec `gpt-5.5` :

```powershell
.\config-chatgpt.cmd
```

Se connecter a ChatGPT :

```powershell
.\login-chatgpt.cmd
```

Tester la connexion :

```powershell
.\test-chatgpt.cmd
```

## CLI rapide

```powershell
.\gitnexus.cmd ask -Question "Resume ce projet en 5 lignes"
.\gitnexus.cmd analyze -Repo D:\CascadeProjects\gitnexus-rs-from-c
.\gitnexus.cmd docs -Repo D:\CascadeProjects\gitnexus-rs-from-c
.\gitnexus.cmd docs -Repo D:\CascadeProjects\gitnexus-rs-from-c -Enrich
```

## Tout verifier

```powershell
.\check-gitnexus.cmd
```

Equivalent :

```powershell
.\gitnexus.cmd check
```

Cette commande relance les validations principales : `chat-ui` lint/tests/build,
UI desktop lint/build, puis `cargo fmt --check` et les tests Rust CLI/MCP/Desktop.
