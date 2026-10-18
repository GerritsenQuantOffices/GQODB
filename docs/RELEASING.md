# Releasing 0.1.0 (procedure for the owner)

Nothing in this procedure has been run for a release. The tag, the GitHub release and
any crates.io publication are the owner's decisions. Every command below that pushes
or publishes says so; do not run those until you have decided to publish.

Placeholders: `<version>` is `0.1.0`; commands run from the repository root on the
commit that will be released (normally `main` after the preparation branches are merged).

## 0. Decisions to make first

1. **crates.io or not.** The manifests keep `publish = false`. If you do not publish, skip
   sections 4 and 8 (crates.io part). The names `gqodb`, `gqodb-types`, `gqodb-blocks`,
   `gqodb-store` and `gqodb-codec` were not registered on crates.io when checked on
   2026-10-17 (HTTP 404 from the crates.io API; a search for "gqodb" returned no crates).
   Names are first come, first served and can be taken at any time.
2. **Signing key for the tag.** No signing key is configured by this procedure. Choose a
   GPG or SSH key and publish its fingerprint somewhere independent of the repository.
3. **Contribution terms** (Apache-2.0 inbound = outbound, or a DCO or CLA) and a
   **trademark note** for the name "GQODB": see `CONTRIBUTING.md`; neither is decided.

## 1. Preconditions

```sh
git status --short                      # must print nothing
git rev-parse --abbrev-ref HEAD         # the release branch, normally main
git log -1 --format=%H                  # note this commit id
```

All CI jobs must be green on this exact commit on GitHub (the `test`, `msrv` and `deny`
jobs; the macOS/Windows entries are allowed to fail and say so). Read the run, do not
assume it.

## 2. Version check

```sh
grep -n '^version' Cargo.toml           # [workspace.package] version = "0.1.0"
grep -n 'rust-version' Cargo.toml       # "1.85"
cargo metadata --format-version 1 --locked --no-deps \
  | python3 -c "import json,sys; [print(p['name'], p['version']) for p in json.load(sys.stdin)['packages']]"
grep -n 'version = "0.1.0", path' crates/*/Cargo.toml   # internal dependencies carry the version
cargo update --workspace --dry-run      # Cargo.lock should not need changes for the workspace crates
```

Then edit by hand:

- `CHANGELOG.md`: replace "not yet released" in the 0.1.0 heading with the release date.
- `docs/RELEASE_NOTES_0.1.0.md`: fill in the date.
- `README.md`: the "0 published releases" figure and the status note must match reality
  after the release (a GitHub release makes it 1; crates.io, if used, changes "no crates.io
  release"). Re-count the test totals if the suites changed since the numbers were written.

Commit these edits (`git commit -s` if you adopt a DCO) and push them through your normal review.

## 3. Full gate on the release commit

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo test --workspace --doc --locked
cargo test -p gqodb-blocks --features zstd-input --all-targets --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo +1.85.0 test --workspace --all-targets --locked     # MSRV; rustup toolchain install 1.85.0 --profile minimal
cargo deny --locked --all-features check                   # cargo install --locked cargo-deny
python3 tests/test_installer.py
python3 results/verify_public_evidence.py
python3 benchmarks/verify_manifest.py
python3 scripts/third_party_licenses.py --check
```

## 4. crates.io dry run (only if publishing to crates.io)

Cargo refuses to package a crate with `publish = false`. Do the dry run in a scratch
copy so the repository keeps its setting:

```sh
scratch=$(mktemp -d)
git archive HEAD | tar -x -C "$scratch"
cd "$scratch"
sed -i 's/^publish = false/publish = true/' Cargo.toml
cargo publish --workspace --dry-run --locked
```

`--workspace` publishes in dependency order (`gqodb-types`, `gqodb-blocks`, `gqodb-store`,
`gqodb-codec`) and verifies each package against its predecessors. Publishing a single
crate by name (`cargo publish -p gqodb-store --dry-run`) fails until `gqodb-blocks` is on
crates.io, which is expected. Inspect what would be uploaded with
`cargo package -p gqodb-blocks --list --locked --allow-dirty`.

Remember before the real publish:

- publishing is irreversible: a version can be yanked (section 9) but never replaced or deleted;
- `documentation = "https://docs.rs/<crate>"` only resolves after the first publish and
  a successful docs.rs build;
- the `=59.3.0` Arrow and Parquet pins are visible to everyone who depends on these crates;
- you need `cargo login` with a crates.io token that is not stored in the repository.

The real publish (irreversible; only after the tag in section 5 exists and CI is green):

```sh
# from the real tree, after flipping publish in a commit you have reviewed:
cargo publish --workspace --locked
```

## 5. Signed annotated tag (local; do not push yet)

```sh
git tag -s v0.1.0 -m "GQODB open layer 0.1.0 (preview)"
git tag -v v0.1.0                        # verify the signature
git show v0.1.0 --stat | head
```

For an SSH signing key set `git config gpg.format ssh` and
`git config user.signingkey ~/.ssh/<key>.pub`, and use an `allowed_signers` file to verify.
If you decide not to sign, use `git tag -a v0.1.0 -m "..."` and say in the release notes that the
tag is unsigned.

Push the tag only when you want it public (this publishes it):

```sh
git push origin v0.1.0
```

## 6. Source archive and checksum

```sh
git archive --format=tar --prefix=GQODB-0.1.0/ v0.1.0 | gzip -n -9 > GQODB-0.1.0.tar.gz
sha256sum GQODB-0.1.0.tar.gz > GQODB-0.1.0.tar.gz.sha256
sha256sum -c GQODB-0.1.0.tar.gz.sha256
# optional: sign the checksum file (GPG shown; an SSH key works with ssh-keygen -Y sign)
gpg --armor --detach-sign GQODB-0.1.0.tar.gz.sha256
```

`gzip -n` leaves out the timestamp, so repeating the command on the same tag gives the
same bytes (checked on the preparation branch with `HEAD` in place of the tag). GitHub's
automatic source archives are generated by GitHub and are not covered by this checksum:
attach your own. Before uploading, scan the archive for anything private:

```sh
tar -tzf GQODB-0.1.0.tar.gz | head -50
# PRIVATE is your own list of strings that must never be public: home-directory paths,
# private host names and addresses, personal e-mail addresses, token prefixes.
PRIVATE='/home/you|private-host-name|you@example.com'
tar -xzOf GQODB-0.1.0.tar.gz | grep -aEn "$PRIVATE" || echo "scan clean"
```

## 7. GitHub release

```sh
gh release create v0.1.0 \
  --verify-tag --prerelease \
  --title "GQODB open layer 0.1.0 (preview)" \
  --notes-file docs/RELEASE_NOTES_0.1.0.md \
  GQODB-0.1.0.tar.gz GQODB-0.1.0.tar.gz.sha256
```

(`--prerelease` marks it as a preview on GitHub. This publishes the release.)

## 8. What to double-check at the end

- Fresh clone: `git clone <url> && git checkout v0.1.0 && ./install.sh --prefix "$(mktemp -d)" --ref v0.1.0`
  completes, and the README commands run as written.
- GitHub repository settings: private vulnerability reporting is on (`SECURITY.md` depends
  on it), the repository description and topics say "preview", branch protection requires
  the `test`, `msrv` and `deny` jobs.
- The first CI run of the macOS and Windows entries: record the result in the changelog
  or the README "Still not measured" row before you claim anything about those platforms.
- crates.io (if used): the pages show the right README, licence, repository and keywords,
  and docs.rs built all four crates.
- No private information in the release notes, the archive or the crate packages
  (`cargo package --list` shows the exact files).

## 9. Yanking and withdrawing

crates.io (reverse dependency order, so that nothing new resolves to a withdrawn dependency):

```sh
cargo yank --version 0.1.0 gqodb-codec
cargo yank --version 0.1.0 gqodb-store
cargo yank --version 0.1.0 gqodb-blocks
cargo yank --version 0.1.0 gqodb-types
# undo: cargo yank --version 0.1.0 --undo <crate>
```

Yanking stops new dependency resolution; it does not delete the files, and existing
`Cargo.lock` files keep working. If a release contains a secret, rotate the secret first;
crates.io staff can remove a crate version only by request (see the crates.io policies).

GitHub: prefer publishing a corrected 0.1.1 and marking the bad release in its notes. Deleting
a tag that others have fetched breaks their checkouts. If you must:
`gh release delete v0.1.0 --yes` and `git push origin :refs/tags/v0.1.0` (both destructive).
