#!/usr/bin/env bash
set -euo pipefail

CLUSTER_NAME="${CLUSTER_NAME:-glum}"
CONTEXT_NAME="${CONTEXT_NAME:-kind-${CLUSTER_NAME}}"
NAMESPACE="${NAMESPACE:-glum}"
SECRET_NAME="${SECRET_NAME:-google-oauth-secret}"
KUBECONFIG_PATH="${KUBECONFIG:-$HOME/.kube/config}"

die() {
  echo "ERROR: $*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "Missing required command: $1"
}

echo "Using kubeconfig: $KUBECONFIG_PATH"

need_cmd kind
need_cmd kubectl
need_cmd curl

export KUBECONFIG="$KUBECONFIG_PATH"

if ! kind get clusters | grep -qx "$CLUSTER_NAME"; then
  die "kind cluster '$CLUSTER_NAME' was not found. Existing clusters: $(kind get clusters | tr '\n' ' ')"
fi

echo "Exporting kubeconfig for kind cluster '$CLUSTER_NAME'..."
kind export kubeconfig --name "$CLUSTER_NAME" >/dev/null

kubectl config use-context "$CONTEXT_NAME" >/dev/null
kubectl config set-context --current --namespace="$NAMESPACE" >/dev/null

current_context="$(kubectl config current-context)"
if [[ "$current_context" != "$CONTEXT_NAME" ]]; then
  die "kubectl is on '$current_context', expected '$CONTEXT_NAME'"
fi

kubectl get namespace "$NAMESPACE" >/dev/null 2>&1 || die "Namespace '$NAMESPACE' was not found"

echo "Verifying Glum app pods in context '$CONTEXT_NAME', namespace '$NAMESPACE'..."
kubectl get deployment backend -n "$NAMESPACE" >/dev/null
kubectl get deployment envoy -n "$NAMESPACE" >/dev/null
kubectl get deployment web -n "$NAMESPACE" >/dev/null

read -rp "Google OAuth Client ID: " GOOGLE_CLIENT_ID
read -rsp "Google OAuth Client Secret: " GOOGLE_CLIENT_SECRET
echo

[[ -n "$GOOGLE_CLIENT_ID" ]] || die "Google OAuth Client ID is required"
[[ -n "$GOOGLE_CLIENT_SECRET" ]] || die "Google OAuth Client Secret is required"

echo "Applying Kubernetes secret '$SECRET_NAME'..."
kubectl create secret generic "$SECRET_NAME" \
  -n "$NAMESPACE" \
  --from-literal=client-id="$GOOGLE_CLIENT_ID" \
  --from-literal=client-secret="$GOOGLE_CLIENT_SECRET" \
  --dry-run=client \
  -o yaml | kubectl apply -f -

kubectl get secret "$SECRET_NAME" -n "$NAMESPACE" >/dev/null

echo "Restarting backend..."
kubectl rollout restart deployment/backend -n "$NAMESPACE" >/dev/null
kubectl rollout status deployment/backend -n "$NAMESPACE" --timeout=120s

echo "Checking auth service through localhost:4000..."
if ! curl -fsS http://localhost:4000/auth/health >/dev/null; then
  cat >&2 <<'MSG'
Auth health check failed on http://localhost:4000/auth/health.
Start the web port-forward in another terminal:

  kubectl port-forward -n glum svc/web 4000:80

Then rerun this script or test login again.
MSG
  exit 1
fi

login_headers="$(curl -fsS -D - http://localhost:4000/auth/google/login -o /dev/null)"
if grep -q "Google+OAuth+is+not+configured" <<<"$login_headers"; then
  die "Backend still says Google OAuth is not configured"
fi

echo
echo "Google OAuth secret is configured in $CONTEXT_NAME/$NAMESPACE."
echo "Open http://localhost:4000 and log in with Google."
