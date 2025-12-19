# Deploy en Fly.io - Guía Paso a Paso

## Archivos creados

- `fly.toml` - Configuración de Fly.io
- `deploy-fly.sh` - Script automatizado de deploy
- `Dockerfile` - Modificado para incluir secrets de Firebase

## Pasos para hacer deploy

### 1. Autenticarse en Fly.io

```bash
flyctl auth login
```

Esto abrirá tu navegador para que inicies sesión.

### 2. Crear la aplicación (primera vez)

```bash
flyctl launch --no-deploy
```

Este comando:
- Lee el archivo `fly.toml`
- Crea la app en Fly.io
- **NO** hace deploy todavía

Si te pregunta si quieres cambiar algo, puedes aceptar los valores por defecto.

### 3. Opción A: Deploy automático (recomendado)

```bash
./deploy-fly.sh
```

Este script hace todo automáticamente:
- Configura todos los secrets
- Hace el deploy
- Muestra comandos útiles

### 3. Opción B: Deploy manual

Si prefieres hacerlo paso a paso:

```bash
# Configurar secrets
flyctl secrets set \
  NOSTR_RELAYS="wss://relay.mostro.network" \
  MOSTRO_PUBKEY="0a537332f2d569059add3fd2e376e1d6b8c1e1b9f7a999ac2592b4afbba74a00" \
  SERVER_PRIVATE_KEY="ccc61d16dfd10fbcca1322fdf5fed6cb1863db4e27030ae164dbcbfcc263154d" \
  FIREBASE_PROJECT_ID="mostro-test" \
  FIREBASE_SERVICE_ACCOUNT_PATH="/secrets/mostro-test-firebase-adminsdk-fbsvc-9da9480201.json" \
  FCM_ENABLED="true" \
  UNIFIEDPUSH_ENABLED="false" \
  SERVER_HOST="0.0.0.0" \
  SERVER_PORT="8080" \
  TOKEN_TTL_HOURS="48" \
  CLEANUP_INTERVAL_HOURS="1" \
  RATE_LIMIT_PER_MINUTE="60" \
  BATCH_DELAY_MS="5000" \
  COOLDOWN_MS="60000" \
  RUST_LOG="debug"

# Hacer deploy
flyctl deploy
```

## Comandos útiles después del deploy

```bash
# Ver estado de la aplicación
flyctl status

# Ver logs en tiempo real
flyctl logs

# Ver logs con filtro
flyctl logs -a mostro-push-server

# Verificar secrets configurados
flyctl secrets list

# Abrir la app en el navegador
flyctl open

# Conectar por SSH a la máquina
flyctl ssh console

# Ver información de la app
flyctl info

# Escalar la aplicación (cambiar recursos)
flyctl scale vm shared-cpu-1x --memory 512

# Detener la aplicación
flyctl apps stop

# Reiniciar la aplicación
flyctl apps restart
```

## Verificar que funciona

Una vez deployado, tu API estará disponible en:
```
https://mostro-push-server.fly.dev
```

Prueba los endpoints:

```bash
# Health check
curl https://mostro-push-server.fly.dev/api/health

# Server info
curl https://mostro-push-server.fly.dev/api/info

# Status
curl https://mostro-push-server.fly.dev/api/status
```

## Re-deploy (actualizaciones)

Para hacer deploy de cambios posteriores:

```bash
flyctl deploy
```

Los secrets ya están configurados, no necesitas volver a configurarlos a menos que cambien.

## Troubleshooting

### Ver logs si algo falla

```bash
flyctl logs --follow
```

### Verificar que los secrets están configurados

```bash
flyctl secrets list
```

### Conectar a la máquina para debugging

```bash
flyctl ssh console
```

### Si el build falla

Verifica que el archivo `secrets/mostro-test-firebase-adminsdk-fbsvc-9da9480201.json` existe en tu proyecto local.

### Cambiar región

Si quieres cambiar la región (por defecto es `gru` - São Paulo):

```bash
# Ver regiones disponibles
flyctl platform regions

# Cambiar región en fly.toml y re-deploy
# Edita fly.toml y cambia primary_region = "scl" (Santiago) u otra
```

## Costos

Fly.io tiene un tier gratuito que incluye:
- 3 máquinas compartidas (256MB RAM)
- 3GB de almacenamiento persistente
- 160GB de tráfico saliente

Tu configuración actual usa 512MB RAM, lo cual está dentro del tier gratuito para 1 máquina.

## Seguridad

⚠️ **IMPORTANTE**: 
- Los secrets están encriptados en Fly.io
- El archivo `.env` NO se sube al servidor (está en `.gitignore`)
- El archivo Firebase service account se incluye en la imagen Docker
- Considera rotar el `SERVER_PRIVATE_KEY` periódicamente

## Monitoreo

Para producción, considera:
- Configurar alertas en Fly.io
- Usar `flyctl logs` para monitorear errores
- Verificar métricas en el dashboard de Fly.io
