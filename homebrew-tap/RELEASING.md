# Releasing `dw-cli` With Bottles

This document describes the intended release sequence once the scaffold in this
directory is copied into the real tap repository `estebanwasinger/homebrew-tap`.

## 1. Release the source archive from `dataweave-py`

Create a GitHub release in `estebanwasinger/dataweave-py`, typically using a
tag such as:

```bash
v1.0.2
```

That release triggers the `Release dw CLI Source Asset` workflow in the main
repository, which uploads:

- `dw-cli-<version>-source.tar.gz`
- `dw-cli-<version>-source.tar.gz.sha256`

## 2. Update the tap formula

In `estebanwasinger/homebrew-tap`:

1. Update `Formula/dw-cli.rb`
2. Set `url` to the new `dataweave-py` release asset
3. Set `sha256` to the new checksum
4. Merge the formula change into the default branch

## 3. Publish the tap release

Create a release in the tap repository using a tag such as:

```bash
dw-cli-1.0.2
```

This release is the upload target for the Homebrew bottles.

## 4. Let the bottle workflow run

The tap repo `Publish Bottles` workflow should:

- check out the default branch
- build the formula with `brew test-bot`
- publish bottles to the tap release
- commit the generated `bottle do` block back to the default branch

The workflow uses the release tag only for the bottle asset URL:

```text
https://github.com/estebanwasinger/homebrew-tap/releases/download/<tag>
```

## 5. Verify the result

After the workflow succeeds:

1. Confirm the tap release contains bottle files
2. Confirm `Formula/dw-cli.rb` on the default branch contains a `bottle do`
   block
3. Confirm a fresh install uses bottles:

```bash
brew tap estebanwasinger/homebrew-tap
brew install dw-cli
dw run "1 to 1"
```
