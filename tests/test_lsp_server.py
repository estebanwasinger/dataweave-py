from __future__ import annotations

from types import SimpleNamespace
from typing import cast

from lsprotocol import types

from dwpy.lsp import server as lsp_server
from dwpy.lsp.engine import EngineCompletionItem


def test_uri_to_path_handles_file_uri() -> None:
    assert lsp_server._uri_to_path("file:///tmp/example.dwl") == "/tmp/example.dwl"


def test_completion_handler_maps_engine_items(monkeypatch) -> None:
    monkeypatch.setattr(lsp_server, "_read_document", lambda _uri: "payload.")
    monkeypatch.setattr(lsp_server, "_uri_to_path", lambda _uri: "/tmp/sample.dwl")

    fake_ls = SimpleNamespace(
        engine=SimpleNamespace(
            complete=lambda **_kwargs: [
                EngineCompletionItem(
                    label="name",
                    kind="field",
                    insert_text="name",
                    detail="Object field",
                    insert_text_format="plain",
                    sort_text="1_name",
                )
            ]
        )
    )

    params = types.CompletionParams(
        text_document=types.TextDocumentIdentifier(uri="file:///tmp/sample.dwl"),
        position=types.Position(line=0, character=8),
    )

    result = lsp_server.completion(cast(lsp_server.DataWeaveLanguageServer, fake_ls), params)
    assert isinstance(result, types.CompletionList)
    assert result.items
    assert result.items[0].label == "name"
    assert result.items[0].kind == types.CompletionItemKind.Field
