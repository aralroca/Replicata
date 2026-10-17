#!/usr/bin/env bash
# Governance gate: every dependency (Rust crates or JS packages) must ship under an OSI-approved
# license (the rule for third-party components of an MIT project). Writes licenses-<kind>.json as
# the audit trail.
#
#   scripts/ci/license-audit.sh rust|js
set -euo pipefail

KIND=${1:?rust|js}
# SPDX ids allowed on their own or inside an expression (e.g. "MIT OR Apache-2.0").
# LGPL-3.0: only @img/sharp-libvips, the unmodified native image library Next.js loads dynamically.
ALLOWED='^(MIT|MIT-0|Apache-2\.0|Apache-2\.0 WITH LLVM-exception|BSD-2-Clause|BSD-3-Clause|ISC|Zlib|0BSD|Unicode-3\.0|Unicode-DFS-2016|CC0-1\.0|BSL-1\.0|MPL-2\.0|Unlicense|CDLA-Permissive-2\.0|BlueOak-1\.0\.0|Python-2\.0|CC-BY-4\.0|OpenSSL|LGPL-3\.0-or-later)$'

# Crates that ship a license file instead of an SPDX id: identify it from the file's header.
license_from_file() {
  case "$(head -c 400 "$1" 2>/dev/null)" in
    *"Apache License"*) echo Apache-2.0 ;;
    *"MIT License"* | *"Permission is hereby granted, free of charge"*) echo MIT ;;
    *) echo UNKNOWN ;;
  esac
}

rust_licenses() {
  cargo metadata --format-version 1 --locked |
    jq -r '.packages[] | [.name, .version, (.license // ""), ((.manifest_path | rtrimstr("Cargo.toml")) + (.license_file // ""))] | join("|")' |
    while IFS="|" read -r name version license file; do
      jq -n --arg n "$name" --arg v "$version" --arg l "${license:-$(license_from_file "$file")}" \
        '{name: $n, version: $v, license: $l}'
    done | jq -s .
}

js_licenses() {
  pnpm licenses list --json --prod |
    jq '[to_entries[] | .key as $license | .value[] | {name, version: (.versions[0]), license: $license}]'
}

# An expression passes when at least one OR-branch is made only of allowed ids.
acceptable() {
  local branch

  while read -r branch; do
    [[ -n $branch ]] || continue
    tr '&' '\n' <<<"${branch// AND /&}" | sed 's/^[( ]*//; s/[) ]*$//' | grep -Evq "$ALLOWED" || return 0
  done < <(sed 's#/#\n#g; s/ OR /\n/g' <<<"$1")

  return 1
}

"${KIND}_licenses" >"licenses-$KIND.json"
violations=$(jq -r '.[] | "\(.license)\t\(.name)@\(.version)"' "licenses-$KIND.json" | sort -u |
  while IFS=$'\t' read -r license package; do acceptable "$license" || echo "$license  $package"; done)
echo "$(jq length "licenses-$KIND.json") $KIND packages audited"
[[ -z $violations ]] || { echo "non-OSI or unknown licenses:"; echo "$violations"; exit 1; }
echo "all licenses OSI-approved"
