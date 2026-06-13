# homebrew-tap

This directory is a scaffold for the separate tap repository
`estebanwasinger/homebrew-tap`.

The intended install flow is:

```bash
brew tap estebanwasinger/homebrew-tap
brew install dw-cli
dw run "1 to 1"
```

## What lives here

- `Formula/dw-cli.rb`: the Homebrew formula for the Rust-native `dw` CLI
- `.github/workflows/tests.yml`: formula audit and build checks
- `.github/workflows/publish.yml`: bottle publishing workflow scaffold

## Release flow

1. Publish a release in `estebanwasinger/dataweave-py`.
2. The `Release dw CLI Source Asset` workflow uploads:
   - `dw-cli-<version>-source.tar.gz`
   - `dw-cli-<version>-source.tar.gz.sha256`
3. Update `Formula/dw-cli.rb` in the tap repo with:
   - the new `url`
   - the new `sha256`
4. Merge the formula change into the tap repo default branch.
5. Create a release in the tap repo for the same formula version, for example
   `dw-cli-1.0.2`.
6. The tap `Publish Bottles` workflow:
   - checks out the default branch, not the tag, so Homebrew can commit the
     generated `bottle do` block back to the repository
   - uploads bottle files to the tap repo release that triggered the workflow
7. Verify the bottle commit lands on the tap default branch and future installs
   fetch bottles instead of rebuilding from source.

## Bottles

Bottle files are published from the tap repository release, not from the
`dataweave-py` repository release.

The split is:

- `dataweave-py` release: source tarball for `Formula/dw-cli.rb`
- `homebrew-tap` release: bottle artifacts built by `brew test-bot --publish`

## Creating the actual tap repository

The safest bootstrap path is:

```bash
brew tap-new estebanwasinger/homebrew-tap
```

Then copy the files from this scaffold into that repository, keeping the tap
name and formula path the same.

The tap repo must allow GitHub Actions to write repository contents so the
bottle workflow can upload release assets and commit the updated formula.

## Refreshing the formula checksum

From the `dataweave-py` repository:

```bash
bash scripts/build_dw_cli_source_archive.sh 1.0.2 HEAD
cat dist/dw-cli-1.0.2-source.tar.gz.sha256
```

Use the printed checksum in `Formula/dw-cli.rb`.
