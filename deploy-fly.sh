#!/bin/bash

# Script para hacer deploy de mostro-push-server en Fly.io
# Asegúrate de estar autenticado: flyctl auth login

set -e

echo "🚀 Iniciando deploy en Fly.io..."

# Verificar que flyctl está instalado
if ! command -v flyctl &> /dev/null; then
    echo "❌ Error: flyctl no está instalado"
    echo "Instala con: curl -L https://fly.io/install.sh | sh"
    exit 1
fi

# Verificar que estás autenticado
if ! flyctl auth whoami &> /dev/null; then
    echo "❌ Error: No estás autenticado en Fly.io"
    echo "Ejecuta: flyctl auth login"
    exit 1
fi

echo "📝 Configurando secrets..."

# Configurar todos los secrets
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

echo "✅ Secrets configurados"

echo "🏗️  Haciendo deploy..."
flyctl deploy

echo "✅ Deploy completado!"
echo ""
echo "📊 Comandos útiles:"
echo "  flyctl status          - Ver estado de la app"
echo "  flyctl logs            - Ver logs en tiempo real"
echo "  flyctl ssh console     - Conectar a la máquina"
echo "  flyctl secrets list    - Ver secrets configurados"
echo "  flyctl open            - Abrir la app en el navegador"
echo ""
echo "🌐 Tu app estará disponible en: https://mostro-push-server.fly.dev"
