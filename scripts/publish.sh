#!/usr/bin/env bash
# Publish every crate in this workspace to crates.io in dependency order.
#
# Path dependencies with a `version` requirement must already exist on
# crates.io before the crate that depends on them can be published — cargo
# resolves those requirements against the registry, not the local path,
# during `cargo publish`. Publishing out of order fails with something like:
#
#   error: failed to select a version for the requirement `hiero-did-registrar = "^0.1.3"`
#   candidate versions found which didn't match: 0.1.0
#
# Run with DRY_RUN=1 ./scripts/publish.sh to verify without uploading.

set -euo pipefail

# Topologically sorted: every crate appears after everything it depends on.
CRATES=(
    core
    utils
    lifecycle
    client
    signer
    method
    hcs
    messages
    anoncreds
    registrar
    resolver
    sdk
)

DRY_RUN_FLAG=""
if [[ "${DRY_RUN:-0}" == "1" ]]; then
    DRY_RUN_FLAG="--dry-run"
fi

for crate in "${CRATES[@]}"; do
    echo "==> Publishing $crate"
    (cd "$crate" && cargo publish $DRY_RUN_FLAG)
    if [[ -z "$DRY_RUN_FLAG" ]]; then
        # Give crates.io's index a moment to update before the next crate's
        # publish tries to resolve it as a path+version dependency.
        sleep 15
    fi
done
