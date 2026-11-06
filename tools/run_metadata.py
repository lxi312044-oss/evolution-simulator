"""Record per-run provenance before execution; verify inputs again on completion."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def command(root, *args):
    result = subprocess.run(args, cwd=root, capture_output=True, text=True)
    return result.stdout.strip() if result.returncode == 0 else None


def source_hashes(root):
    paths = [root / "Cargo.toml", root / "Cargo.lock"]
    paths += sorted((root / "crates").rglob("*.rs"))
    paths += sorted((root / "crates").rglob("Cargo.toml"))
    paths += [root / "tools/run_phase0759.ps1", root / "tools/run_metadata.py"]
    return {p.relative_to(root).as_posix(): sha256(p) for p in paths}


def record(stage, root, config, binary, output, ticks, interval):
    root, config, binary, output = map(lambda p: Path(p).resolve(), (root, config, binary, output))
    if ticks <= 0 or interval <= 0:
        raise ValueError("ticks and interval must be positive")
    inputs = dict(config_sha256=sha256(config), binary_sha256=sha256(binary),
                  source_sha256=source_hashes(root), ticks=ticks, interval=interval,
                  seed=json.loads(config.read_text(encoding="utf-8-sig"))["seed"])
    now = datetime.now(timezone.utc).isoformat()
    if stage == "begin":
        if output.exists():
            raise ValueError(f"Run metadata already exists: {output}; use a new output directory")
        commit = command(root, "git", "rev-parse", "HEAD")
        status = command(root, "git", "status", "--porcelain", "--untracked-files=normal") if commit else None
        data = dict(schema_version=1, status="running", started_at=now, inputs=inputs,
                    git_commit=commit, git_dirty=bool(status) if status is not None else None,
                    git_status=status, rustc=command(root, "rustc", "--version"),
                    cargo=command(root, "cargo", "--version"), python=sys.version,
                    config_file=config.name, example="toxin_evolution_0758")
    else:
        data = json.loads(output.read_text(encoding="utf-8"))
        if data["status"] != "running" or data["inputs"] != inputs:
            raise ValueError("Run inputs changed or run was already completed")
        data.update(status="completed", completed_at=now)
    temporary = output.with_suffix(".tmp")
    temporary.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    temporary.replace(output)
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("stage", choices=["begin", "complete"])
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--ticks", type=int, required=True)
    parser.add_argument("--interval", type=int, required=True)
    args = parser.parse_args()
    try:
        record(**vars(args))
    except (ValueError, OSError, KeyError) as error:
        parser.exit(2, f"Metadata failure: {error}\n")


if __name__ == "__main__":
    main()
