"""Build audit-enabled current and pinned baseline WASM in a fresh output directory."""
import io
from pathlib import Path
import re
import subprocess
import sys
import tarfile
import tempfile

root = Path(__file__).resolve().parent.parent
baseline_commit = "787034ab6de01e1e9d0f585a1f767117fa3cab0d"
out = Path(sys.argv[1]).resolve()
out.mkdir(parents=True)
scratch = root / "target"
scratch.mkdir(exist_ok=True)
baseline = Path(tempfile.mkdtemp(prefix="stage3-audit-", dir=scratch))
archive = subprocess.check_output(["git", "archive", baseline_commit, "crates", "Cargo.toml", "Cargo.lock"], cwd=root)
with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
    tar.extractall(baseline, filter="data")

# Insert only the test feature, serializers and read-only audit export.
# The baseline ecology is taken directly from the pinned Git object.
for file in ("Cargo.lock", "crates/sim-core/Cargo.toml", "crates/sim-wasm/Cargo.toml"):
    (baseline / file).write_bytes((root / file).read_bytes())

derive = '#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]\n'
for source in (root / "crates/sim-core/src").rglob("*.rs"):
    target = baseline / source.relative_to(root)
    if not target.is_file():
        continue
    text = target.read_text(encoding="utf-8")
    for declaration in re.findall(re.escape(derive) + r"([^\n]+)", source.read_text(encoding="utf-8")):
        assert text.count(declaration) == 1, (source, declaration)
        text = text.replace(declaration, derive + declaration, 1)
    target.write_text(text, encoding="utf-8")

world = Path("crates/sim-core/src/world")
audit = (root / world / "state_audit.rs").read_text(encoding="utf-8")
audit = audit.replace("legacy: bool", "_legacy: bool").replace("            animal_toxins,\n", "")
audit = audit[:audit.index("        if legacy {")] + audit[audit.index("        bytes\n"):]
audit = audit[:audit.index("#[cfg(test)]")]
(baseline / world / "state_audit.rs").write_text(audit, encoding="utf-8")
module = baseline / world / "mod.rs"
module.write_text('#[cfg(feature = "state-audit")]\nmod state_audit;\n' + module.read_text(encoding="utf-8"), encoding="utf-8")

wasm = Path("crates/sim-wasm/src/lib.rs")
source = (root / wasm).read_text(encoding="utf-8")
marker = "    pub fn tick_count("
export = source[source.index("    /// Test-only snapshot."):source.index(marker)]
target = baseline / wasm
target.write_text(target.read_text(encoding="utf-8").replace(marker, export + marker), encoding="utf-8")

for label, cwd in (("current", root), ("baseline", baseline)):
    subprocess.run(["wasm-pack", "build", "crates/sim-wasm", "--release", "--target", "web",
                    "--no-opt", "--mode", "no-install", "--out-dir", str(out / label),
                    "--", "--features", "state-audit", "--locked"], cwd=cwd, check=True)
