import json

import pytest

from scripts.release_version import (
    bump_patch,
    normalize_release_tag,
    prepare_bump,
    resolve_version,
)


def test_normalize_release_tag_accepts_optional_v_prefix():
    assert normalize_release_tag("v1.2.3") == "1.2.3"
    assert normalize_release_tag("1.2.3") == "1.2.3"


@pytest.mark.parametrize("tag", ["", "vv1.2.3", "01.2.3", "1.2", "1.2.3-rc.1"])
def test_normalize_release_tag_rejects_non_stable_semver(tag):
    with pytest.raises(ValueError):
        normalize_release_tag(tag)


def test_release_event_uses_tag_and_resolves_next_patch(tmp_path):
    pyproject = tmp_path / "pyproject.toml"
    pyproject.write_text('[project]\nversion = "1.2.2"\n', encoding="utf-8")

    result = resolve_version("release", "v1.2.3", pyproject)

    assert result == {
        "version": "1.2.3",
        "next_version": "1.2.4",
        "publish_target": "pypi",
    }


def test_testpypi_dispatch_uses_current_project_version_without_bump(tmp_path):
    pyproject = tmp_path / "pyproject.toml"
    pyproject.write_text('[project]\nversion = "1.2.3"\n', encoding="utf-8")

    result = resolve_version("workflow_dispatch", "", pyproject)

    assert result == {
        "version": "1.2.3",
        "next_version": "",
        "publish_target": "testpypi",
    }


def test_prepare_bump_does_not_downgrade_a_branch_that_already_advanced(tmp_path):
    pyproject = tmp_path / "pyproject.toml"
    npm_package = tmp_path / "package.json"
    pyproject.write_text('[project]\nversion = "1.2.5"\n', encoding="utf-8")
    npm_package.write_text(json.dumps({"version": "1.2.4"}), encoding="utf-8")

    result = prepare_bump("1.2.4", pyproject, npm_package)

    assert result == {"version": "1.2.5", "changed": "true"}


def test_prepare_bump_is_idempotent_when_manifests_are_already_current(tmp_path):
    pyproject = tmp_path / "pyproject.toml"
    npm_package = tmp_path / "package.json"
    pyproject.write_text('[project]\nversion = "1.2.4"\n', encoding="utf-8")
    npm_package.write_text(json.dumps({"version": "1.2.4"}), encoding="utf-8")

    result = prepare_bump("1.2.4", pyproject, npm_package)

    assert result == {"version": "1.2.4", "changed": "false"}


def test_bump_patch_advances_only_the_patch_component():
    assert bump_patch("1.2.9") == "1.2.10"
