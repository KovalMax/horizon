#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
dir=${HORIZON_CERT_DIR:-.local/certs}
umask 077
mkdir -p "$dir"
tmp=$(mktemp -d "$dir/.generate.XXXXXXXX")
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT HUP INT TERM
openssl req -x509 -newkey rsa:3072 -sha256 -days 365 -nodes \
  -keyout "$tmp/local-ca-key.pem" -out "$tmp/local-ca.pem" \
  -subj '/CN=Horizon Local Development CA' \
  -addext 'basicConstraints=critical,CA:TRUE' \
  -addext 'keyUsage=critical,keyCertSign,cRLSign' >/dev/null 2>&1
openssl req -new -newkey rsa:2048 -nodes \
  -keyout "$tmp/horizon-key.pem" -out "$tmp/horizon.csr" \
  -subj '/CN=horizon.localhost' >/dev/null 2>&1
cat > "$tmp/extensions.cnf" <<'EOF'
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectAltName=DNS:horizon.localhost,DNS:api.horizon.localhost
EOF
openssl x509 -req -in "$tmp/horizon.csr" -CA "$tmp/local-ca.pem" \
  -CAkey "$tmp/local-ca-key.pem" -CAcreateserial -days 365 -sha256 \
  -extfile "$tmp/extensions.cnf" -out "$tmp/horizon.pem" >/dev/null 2>&1
for file in local-ca-key.pem local-ca.pem horizon-key.pem horizon.pem horizon.csr extensions.cnf local-ca.srl; do
  [ ! -f "$tmp/$file" ] || mv -f "$tmp/$file" "$dir/$file"
done
echo "Local development certificates created in $dir."
echo "Trust the new $dir/local-ca.pem; remove any old Horizon CA trust."
echo "Restart the local stack with: make restart"
