from __future__ import annotations

from pathlib import Path
from urllib.parse import unquote, urlparse

from lsprotocol import types
from pygls.lsp.server import LanguageServer

from .engine import DataWeaveLanguageEngine


SERVER_NAME = "dwpy-lsp"
SERVER_VERSION = "0.1.0"


class DataWeaveLanguageServer(LanguageServer):
    def __init__(self) -> None:
        super().__init__(name=SERVER_NAME, version=SERVER_VERSION)
        self.engine = DataWeaveLanguageEngine()


server = DataWeaveLanguageServer()


_KIND_MAP = {
    "text": types.CompletionItemKind.Text,
    "method": types.CompletionItemKind.Method,
    "function": types.CompletionItemKind.Function,
    "constructor": types.CompletionItemKind.Constructor,
    "field": types.CompletionItemKind.Field,
    "variable": types.CompletionItemKind.Variable,
    "class": types.CompletionItemKind.Class,
    "interface": types.CompletionItemKind.Interface,
    "module": types.CompletionItemKind.Module,
    "property": types.CompletionItemKind.Property,
    "unit": types.CompletionItemKind.Unit,
    "value": types.CompletionItemKind.Value,
    "enum": types.CompletionItemKind.Enum,
    "keyword": types.CompletionItemKind.Keyword,
    "snippet": types.CompletionItemKind.Snippet,
    "color": types.CompletionItemKind.Color,
    "file": types.CompletionItemKind.File,
    "reference": types.CompletionItemKind.Reference,
    "folder": types.CompletionItemKind.Folder,
    "enumMember": types.CompletionItemKind.EnumMember,
    "constant": types.CompletionItemKind.Constant,
    "struct": types.CompletionItemKind.Struct,
    "event": types.CompletionItemKind.Event,
    "operator": types.CompletionItemKind.Operator,
    "typeParameter": types.CompletionItemKind.TypeParameter,
}


def _uri_to_path(uri: str) -> str | None:
    parsed = urlparse(uri)
    if parsed.scheme != "file":
        return None

    path = unquote(parsed.path)
    if parsed.netloc and path:
        # Handles file://hostname/path style URIs.
        path = f"//{parsed.netloc}{path}"

    if len(path) >= 3 and path[0] == "/" and path[2] == ":":
        # Windows drive letter URI -> local path.
        path = path[1:]

    return path


def _read_document(uri: str) -> str:
    try:
        document = server.workspace.get_text_document(uri)
    except Exception:
        document = None

    if document is not None:
        return document.source

    path = _uri_to_path(uri)
    if path is None:
        return ""
    try:
        return Path(path).read_text(encoding="utf-8")
    except Exception:
        return ""


@server.feature(
    types.TEXT_DOCUMENT_COMPLETION,
    types.CompletionOptions(trigger_characters=[".", ":", "("], resolve_provider=False),
)
def completion(
    ls: DataWeaveLanguageServer,
    params: types.CompletionParams,
) -> types.CompletionList:
    uri = params.text_document.uri
    source = _read_document(uri)
    path = _uri_to_path(uri)

    items = ls.engine.complete(
        script=source,
        line=params.position.line,
        column=params.position.character,
        document_path=path,
    )

    response_items: list[types.CompletionItem] = []
    for item in items:
        response_items.append(
            types.CompletionItem(
                label=item.label,
                kind=_KIND_MAP.get(item.kind, types.CompletionItemKind.Text),
                detail=item.detail,
                documentation=item.documentation,
                insert_text=item.insert_text or item.label,
                insert_text_format=(
                    types.InsertTextFormat.Snippet
                    if item.insert_text_format == "snippet"
                    else types.InsertTextFormat.PlainText
                ),
                sort_text=item.sort_text,
            )
        )

    return types.CompletionList(is_incomplete=False, items=response_items)


@server.feature(types.TEXT_DOCUMENT_HOVER)
def hover(
    ls: DataWeaveLanguageServer,
    params: types.HoverParams,
) -> types.Hover | None:
    uri = params.text_document.uri
    source = _read_document(uri)
    path = _uri_to_path(uri)

    result = ls.engine.hover(
        script=source,
        line=params.position.line,
        column=params.position.character,
        document_path=path,
    )
    if result is None:
        return None

    return types.Hover(
        contents=types.MarkupContent(kind=types.MarkupKind.Markdown, value=result.contents)
    )


@server.feature(
    types.TEXT_DOCUMENT_SIGNATURE_HELP,
    types.SignatureHelpOptions(trigger_characters=["(", ","], retrigger_characters=[","]),
)
def signature_help(
    ls: DataWeaveLanguageServer,
    params: types.SignatureHelpParams,
) -> types.SignatureHelp | None:
    uri = params.text_document.uri
    source = _read_document(uri)
    path = _uri_to_path(uri)

    result = ls.engine.signature_help(
        script=source,
        line=params.position.line,
        column=params.position.character,
        document_path=path,
    )
    if result is None:
        return None

    signatures: list[types.SignatureInformation] = []
    for signature in result.signatures:
        signatures.append(
            types.SignatureInformation(
                label=signature.label,
                documentation=signature.documentation,
                parameters=[types.ParameterInformation(label=parameter) for parameter in signature.parameters],
            )
        )

    return types.SignatureHelp(
        signatures=signatures,
        active_signature=result.active_signature,
        active_parameter=result.active_parameter,
    )


def main() -> None:
    server.start_io()


if __name__ == "__main__":
    main()
