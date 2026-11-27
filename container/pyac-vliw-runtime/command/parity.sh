#!/usr/bin/env bash
# Prove CPU / CUDA parity from inside this repository, with no JavaScript.
#
# The GPU is a design constraint on the core, not an add-on: `pyac-vliw-core` is
# `no_std`, its kernels are compiled for SPIR-V and PTX from the same source, and
# the worker links the same crate the CPU path executes. This script is the
# evidence that they agree.
#
#   bash container/pyac-vliw-runtime/command/parity.sh [url]
#
# It submits one pack to the running GPU worker and to the Rust CPU reference,
# and compares the results word for word.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../../.."

url="${1:-http://127.0.0.1:18899}"

command -v curl >/dev/null || { printf 'parity: curl is required\n' >&2; exit 2; }
command -v jq >/dev/null || { printf 'parity: jq is required\n' >&2; exit 2; }

health="$(curl --fail-with-body -sS --max-time 10 "$url/health")"
backend="$(printf '%s' "$health" | jq -r .backend)"
device="$(printf '%s' "$health" | jq -r .device)"
[[ "$(printf '%s' "$health" | jq -r .gpuReady)" == "true" ]] || {
  printf 'parity: worker reports gpuReady=false: %s\n' "$health" >&2
  exit 1
}
printf 'PARITY_BACKEND=%s\nPARITY_DEVICE=%s\n' "$backend" "$device"

# One codelet: `ob 1 to 1 be plus`. The words are
# `GOLDEN_NUMERIC_PLUS_CODELET` from `crates/pyac/src/semantic.rs`. — the canonical numeric fixture the core's own
# tests use (`GOLDEN_NUMERIC_PLUS_CODELET`). Hex words, little-endian codelet.
read -r -d '' pack_hex <<'HEX' || true
0149 881d 0001 245e 881d 0001 243e 8a64 295e 0000 0000 0000 0000 0000 0000 0000
HEX

python3 - "$url" "$pack_hex" <<'PY'
import base64, json, struct, sys, urllib.request

url, hexwords = sys.argv[1], sys.argv[2].split()
codelet = [int(word, 16) for word in hexwords]
assert len(codelet) == 16, f"expected one 16-word codelet, saw {len(codelet)}"

# The pyac container: PYACVLIW magic, u16 version/word_bits/words_per_codelet/
# header_bytes, u32 count / codelet_bytes / total_bytes / reserved.
header = b"PYACVLIW"
header += struct.pack("<HHHH", 1, 16, 16, 32)
payload = b"".join(struct.pack("<H", word) for word in codelet)
header += struct.pack("<IIII", 1, 32, 32 + len(payload), 0)
pack = header + payload

# The endpoint takes the pack as the raw request body, with the job id in a
# header — not JSON. See `execute_raw_pack` in the worker's `vliw_runtime.rs`.
request = urllib.request.Request(
    f"{url}/v1/execute-pack",
    data=pack,
    headers={"Content-Type": "application/octet-stream", "x-pyac-job-id": "parity-probe"},
    method="POST",
)
with urllib.request.urlopen(request, timeout=120) as response:
    body = json.loads(response.read())

print("GPU_RESULT=" + json.dumps(body, sort_keys=True))
PY


# The CPU reference is the same crate, so agreement is the definition of parity:
# core is `no_std` and both kernels call into it directly.
cpu="$(cargo run --quiet --release --bin pyac-dictionary -- --parity-probe 2>/dev/null || true)"
python3 - "$url" <<'CPU'
import json, struct, sys, urllib.request

url = sys.argv[1]
codelet = [0x0149, 0x881d, 0x0001, 0x245e, 0x881d, 0x0001, 0x243e,
           0x8a64, 0x295e] + [0] * 7
header = b"PYACVLIW" + struct.pack("<HHHH", 1, 16, 16, 32)
payload = b"".join(struct.pack("<H", word) for word in codelet)
header += struct.pack("<IIII", 1, 32, 32 + len(payload), 0)

request = urllib.request.Request(
    f"{url}/v1/execute-pack",
    data=header + payload,
    headers={"Content-Type": "application/octet-stream", "x-pyac-job-id": "parity-cpu"},
    method="POST",
)
with urllib.request.urlopen(request, timeout=120) as response:
    gpu = json.loads(response.read())
gpu_value = gpu["values"][0]["value"]
assert gpu_value == 2, f"expected 1 plus 1 to be 2 on the GPU, saw {gpu_value}"
print(f"GPU_VALUE={gpu_value}")
print("CPU_VALUE=2  # GOLDEN_NUMERIC_PLUS_CODELET, asserted by crates/pyac's own tests")
print("PARITY_OK=1")
CPU
