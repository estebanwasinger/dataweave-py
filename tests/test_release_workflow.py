from pathlib import Path

import yaml


def test_pypi_publish_downloads_only_python_distribution_artifacts():
    workflow_path = (
        Path(__file__).resolve().parents[1] / ".github" / "workflows" / "publish.yml"
    )
    workflow = yaml.safe_load(workflow_path.read_text(encoding="utf-8"))
    download_steps = [
        step
        for step in workflow["jobs"]["publish-python"]["steps"]
        if step.get("uses", "").startswith("actions/download-artifact@")
    ]

    artifact_filters = [
        step["with"].get("name") or step["with"].get("pattern")
        for step in download_steps
    ]
    assert artifact_filters == ["sdist", "wheels-*"]
