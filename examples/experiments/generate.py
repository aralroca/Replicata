"""Generates spec.json / publish.json for the demo experiments.

The experiments run on the official `python` image pinned by digest, with the code inline
and only the standard library (no BLAS), so results are bit-identical on arm64 and x86_64.
Run: python3 examples/experiments/generate.py
"""

import json
from pathlib import Path

ROOT = Path(__file__).parent
PYTHON = "sha256:70729b46c69b4f1e97c4822c1af3df53a1476cf5ddc6c087c0c10bc3a5678c2f"  # python:3.13-slim (multi-arch)

PI = """import json, os, random
seed = int(os.environ["REPLICATA_SEED"]); n = int(os.environ["SAMPLES"])
rng = random.Random(seed)
inside = sum(1 for _ in range(n) if rng.random() ** 2 + rng.random() ** 2 <= 1.0)
json.dump({"samples": n, "inside": inside, "pi": 4 * inside / n, "seed": seed}, open("/out/result.json", "w"), sort_keys=True)
"""

LORENZ = """import os
# Lorenz attractor with exact integer fixed-point arithmetic (scale 10^9): bit-identical on any CPU.
S = 10 ** 9; dt = S // 1000; sigma, rho, beta = 10 * S, 28 * S, 8 * S // 3
x, y, z = S, S, S
rows = ["step,x,y,z"]
for step in range(1, int(os.environ["STEPS"]) + 1):
    dx = sigma * (y - x) // S; dy = (x * (rho - z) // S) - y; dz = (x * y // S) - (beta * z // S)
    x, y, z = x + dx * dt // S, y + dy * dt // S, z + dz * dt // S
    if step % 100 == 0: rows.append(f"{step},{x},{y},{z}")
open("/out/trajectory.csv", "w").write("\\n".join(rows) + "\\n")
"""

NON_REPRODUCIBLE = """import os, time
# Deliberately NOT reproducible: depends on the clock and on OS randomness.
open("/out/result.txt", "w").write(f"{time.time_ns()} {os.urandom(16).hex()}\\n")
"""


def spec(title, description, code, env):
    return {
        "version": 1,
        "title": title,
        "description": description,
        "image": "python",
        "image_digest": PYTHON,
        "cmd": ["python3", "-c", code],
        "env": env,
        "inputs": [],
        "cpus": 1,
        "mem_mb": 256,
        "timeout_s": 120,
        "seed": 42,
    }


SPECS = {
    "pi-montecarlo": spec(
        "Monte-Carlo estimate of π",
        "Estimate π from 1,000,000 seeded random points (Python stdlib Mersenne Twister, no BLAS).",
        PI,
        {"SAMPLES": "1000000"},
    ),
    "lorenz-int": spec(
        "Lorenz attractor in integer arithmetic",
        "5,000 Euler steps of the Lorenz system in exact fixed-point integers; trajectory every 100 steps.",
        LORENZ,
        {"STEPS": "5000"},
    ),
    "non-reproducible": spec(
        "A non-reproducible experiment",
        "Reads the clock and OS randomness, so independent replications disagree: shows the Unresolved path.",
        NON_REPRODUCIBLE,
        {},
    ),
}


def write(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")


for name, body in SPECS.items():
    folder = ROOT / name
    folder.mkdir(exist_ok=True)
    write(folder / "spec.json", body)
    write(
        folder / "publish.json",
        {
            "spec": body,
            "reward": 3_000_000,
            "bond": 1_000_000,
            "k": 3,
            "max_claims": 3,
            "commit_secs": 300,
            "reveal_secs": 300,
            "access_price": 10_000,
            "author_share_bps": 7_000,
        },
    )
    print(f"wrote {folder}")
