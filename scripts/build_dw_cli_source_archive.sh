#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
version="${1:-}"
ref="${2:-HEAD}"

if [[ -z "${version}" ]]; then
  version="$(python3 - <<'PY'
from pathlib import Path
import re

content = Path("pyproject.toml").read_text(encoding="utf-8")
match = re.search(r'^version = "([^"]+)"$', content, re.MULTILINE)
if not match:
    raise SystemExit("Unable to determine project version from pyproject.toml")
print(match.group(1))
PY
)"
fi

prefix="dw-cli-${version}-source"
dist_dir="${repo_root}/dist"
archive_path="${dist_dir}/${prefix}.tar.gz"
sha_path="${archive_path}.sha256"

mkdir -p "${dist_dir}"

cd "${repo_root}"

python3 - <<'PY' "${repo_root}" "${archive_path}" "${prefix}" "${ref}"
import gzip
import hashlib
import os
import pathlib
import stat
import subprocess
import sys
import tarfile

repo_root = pathlib.Path(sys.argv[1])
archive_path = pathlib.Path(sys.argv[2])
prefix = sys.argv[3]
ref = sys.argv[4]
excluded_prefixes = (
    "dist/",
    "homebrew-tap/",
    "target/",
    ".venv/",
    ".uv-cache/",
)

def list_files() -> list[str]:
    if ref == "HEAD":
        output = subprocess.check_output(
            ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
            cwd=repo_root,
        )
        return sorted(
            path
            for path in output.decode("utf-8").split("\0")
            if path and not path.startswith(excluded_prefixes)
        )

    output = subprocess.check_output(
        ["git", "ls-tree", "-r", "--name-only", "-z", ref],
        cwd=repo_root,
    )
    return sorted(
        path
        for path in output.decode("utf-8").split("\0")
        if path and not path.startswith(excluded_prefixes)
    )

fixed_mtime = 1704067200

with archive_path.open("wb") as raw_file:
    with gzip.GzipFile(filename="", mode="wb", fileobj=raw_file, mtime=0) as gzip_file:
        with tarfile.open(fileobj=gzip_file, mode="w") as tar:
            for relative_name in list_files():
                source_path = repo_root / relative_name
                if not source_path.is_file():
                    continue

                tar_info = tar.gettarinfo(str(source_path), arcname=f"{prefix}/{relative_name}")
                tar_info.uid = 0
                tar_info.gid = 0
                tar_info.uname = ""
                tar_info.gname = ""
                tar_info.mtime = fixed_mtime
                tar_info.mode = stat.S_IMODE(os.lstat(source_path).st_mode)
                with source_path.open("rb") as source_file:
                    tar.addfile(tar_info, source_file)

sha256 = hashlib.sha256(archive_path.read_bytes()).hexdigest()
(archive_path.parent / f"{archive_path.name}.sha256").write_text(f"{sha256}\n", encoding="utf-8")
PY

echo "Created ${archive_path}"
echo "SHA256 $(cat "${sha_path}")"
