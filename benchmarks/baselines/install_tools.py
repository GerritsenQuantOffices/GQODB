#!/usr/bin/env python3
"""Install the B17 baseline tools into a directory OUTSIDE the repository, checksum-verified.

  install_tools.py --dest DIR [--dry-run] [--only clickhouse|python]

* ClickHouse: the official static tarball of one pinned LTS release from GitHub releases.
  Both the sha256 pinned below and the release's own .sha512 file must match; only the
  `clickhouse` binary is extracted.
* DuckDB and pyarrow: pinned wheels from PyPI, installed with `pip --no-index --no-deps` into
  a fresh virtual environment under DIR. The sha256 of each wheel must equal the pin below
  and the digest PyPI reports for it. No script from the network is ever executed.

Nothing is installed into the repository, the system or the user's site-packages. Add
DIR/venv/bin/python as B17_PYTHON and DIR/bin/clickhouse as B17_CLICKHOUSE.
"""
import argparse
import hashlib
import json
import os
import subprocess
import sys
import tarfile
import urllib.request

CLICKHOUSE = {
    "version": "26.3.54.6",
    "tag": "v26.3.54.6-lts",
    "member": "clickhouse-common-static-26.3.54.6/usr/bin/clickhouse",
    "tgz": "clickhouse-common-static-26.3.54.6-amd64.tgz",
    "sha256": "0a704c18b1a93ee5aaa34c9949c67a230d10b3852713d6b69d7ec45ce8218e2f",
    "sha512": "677a02d63e0a62de618d2daf4e6dc5280a29fab7c58b5d5ac33327567c425398ae5e97e46e3eb341093389df60c49d7aaab046d9578f3546d475f265d1e645b2",
}
RELEASES = "https://github.com/ClickHouse/ClickHouse/releases/download"
WHEELS = {  # package, version, wheel tag -> sha256 (PyPI x86_64 manylinux wheels)
    ("duckdb", "1.5.6", "cp311"): "73b108c04c932b36c2fa4e41110cc1c3c8cd510eb49f065f92d050be8e6929fd",
    ("duckdb", "1.5.6", "cp312"): "bc9619ed7d4ffa117b5155d84b44794366bb6635178d78ed5e13a6024845c757",
    ("duckdb", "1.5.6", "cp313"): "644f54ce99b3b61844bc9a3fe80e0aecb1ea4084b1fffc4396d1569db6111679",
    ("duckdb", "1.5.6", "cp314"): "19c5e485e59613b8878d1670bcaa7a010f53c5a4da5ae8e08863e5e529ca6182",
    ("pyarrow", "26.0.0", "cp311"): "6e89dee53aaeb50505ed6152ea55bc7ddfd4f4df264f5427ea255288d8f0e580",
    ("pyarrow", "26.0.0", "cp312"): "6a628922ba20705fa964ca73e4ef959c2fb2f14b9bbec5589a6a1e68e6257c85",
    ("pyarrow", "26.0.0", "cp313"): "240bd18a7487f8767616a948a69dd4e740a8bc36a1c9da49e4dc9a32c5c2faed",
    ("pyarrow", "26.0.0", "cp314"): "ab6914db225d7f399652ae1f08588dfbc9efe617612715701e3d9d5cfa5ca19f",
}


def fetch(url, destination):
    request = urllib.request.Request(url, headers={"User-Agent": "gqodb-b17-installer"})
    with urllib.request.urlopen(request, timeout=600) as response, open(destination, "wb") as out:
        while chunk := response.read(1 << 20):
            out.write(chunk)


def digest(path, algorithm):
    h = hashlib.new(algorithm)
    with open(path, "rb") as handle:
        while chunk := handle.read(1 << 20):
            h.update(chunk)
    return h.hexdigest()


def require(condition, message):
    if not condition:
        raise SystemExit(f"REFUSED: {message}")


def install_clickhouse(dest, download):
    spec = CLICKHOUSE
    url = f"{RELEASES}/{spec['tag']}/{spec['tgz']}"
    print(f"clickhouse {spec['version']}: {url}")
    if not download:
        return None
    os.makedirs(os.path.join(dest, "dl"), exist_ok=True)
    os.makedirs(os.path.join(dest, "bin"), exist_ok=True)
    tgz = os.path.join(dest, "dl", spec["tgz"])
    fetch(url, tgz)
    fetch(url + ".sha512", tgz + ".sha512")
    require(digest(tgz, "sha256") == spec["sha256"], "ClickHouse tarball sha256 differs from the pin")
    declared = open(tgz + ".sha512").read().split()[0]
    require(declared == spec["sha512"] == digest(tgz, "sha512"), "ClickHouse tarball sha512 mismatch")
    target = os.path.join(dest, "bin", "clickhouse")
    with tarfile.open(tgz) as archive:
        member = archive.getmember(spec["member"])
        require(member.isfile(), "unexpected tar member type")
        with archive.extractfile(member) as source, open(target, "wb") as out:
            while chunk := source.read(1 << 20):
                out.write(chunk)
    os.chmod(target, 0o755)
    version = subprocess.run([target, "local", "--version"], capture_output=True, text=True).stdout.strip()
    require(spec["version"] in version, f"unexpected ClickHouse version {version!r}")
    return {"clickhouse": spec["version"], "clickhouse_tgz_sha256": spec["sha256"], "clickhouse_reported": version}


def pypi_wheel(package, version, tag):
    with urllib.request.urlopen(f"https://pypi.org/pypi/{package}/{version}/json", timeout=60) as response:
        files = json.load(response)["urls"]
    for item in files:
        name = item["filename"]
        if f"-{tag}-{tag}-manylinux" in name and "x86_64" in name:
            return name, item["url"], item["digests"]["sha256"]
    raise SystemExit(f"no x86_64 manylinux wheel of {package} {version} for {tag}")


def install_python(dest, download):
    tag = f"cp{sys.version_info.major}{sys.version_info.minor}"
    print(f"python tag {tag}: duckdb 1.5.6, pyarrow 26.0.0 into {os.path.join(dest, 'venv')}")
    if not download:
        return None
    require(all((p, v, tag) in WHEELS for p, v in (("duckdb", "1.5.6"), ("pyarrow", "26.0.0"))),
            f"no pinned wheel digest for Python tag {tag}")
    wheels = []
    os.makedirs(os.path.join(dest, "dl"), exist_ok=True)
    for package, version in (("duckdb", "1.5.6"), ("pyarrow", "26.0.0")):
        name, url, reported = pypi_wheel(package, version, tag)
        path = os.path.join(dest, "dl", name)
        fetch(url, path)
        actual = digest(path, "sha256")
        require(actual == WHEELS[(package, version, tag)] == reported, f"{name}: sha256 mismatch against pin or PyPI")
        wheels.append(path)
    venv = os.path.join(dest, "venv")
    subprocess.run([sys.executable, "-m", "venv", venv], check=True)
    subprocess.run([os.path.join(venv, "bin", "pip"), "install", "--no-deps", "--no-index", *wheels], check=True)
    versions = subprocess.run(
        [os.path.join(venv, "bin", "python"), "-c", "import duckdb,pyarrow;print(duckdb.__version__,pyarrow.__version__)"],
        capture_output=True, text=True, check=True).stdout.split()
    return {"duckdb": versions[0], "pyarrow": versions[1], "python": sys.version.split()[0]}


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--dest", required=True)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--only", choices=["clickhouse", "python"])
    args = parser.parse_args()
    dest = os.path.abspath(args.dest)
    repo = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
    require(not (dest + os.sep).startswith(repo + os.sep), "the destination must be outside the repository")
    info = {}
    if args.only in (None, "clickhouse"):
        info.update(install_clickhouse(dest, not args.dry_run) or {})
    if args.only in (None, "python"):
        info.update(install_python(dest, not args.dry_run) or {})
    if not args.dry_run:
        with open(os.path.join(dest, "TOOLS.json"), "w") as handle:
            json.dump(info, handle, indent=2)
        print(json.dumps(info, indent=2))
        print(f"export B17_PYTHON={dest}/venv/bin/python B17_CLICKHOUSE={dest}/bin/clickhouse")


if __name__ == "__main__":
    main()
