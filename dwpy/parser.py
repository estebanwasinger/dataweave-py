from __future__ import annotations

import re
from dataclasses import dataclass
from typing import List, Optional, Sequence, Tuple


class ParseError(ValueError):
    pass


@dataclass
class VarDeclaration:
    name: str
    expression: "Expression"


@dataclass
class ImportDirective:
    raw: str


@dataclass
class Header:
    version: str
    output: Optional[str]
    imports: List[ImportDirective]
    variables: List[VarDeclaration]




@dataclass
class Script:
    header: Header
    body: "Expression"


class Expression:
    pass


@dataclass
class Parameter:
    name: str
    default: Optional["Expression"] = None


@dataclass
class LambdaExpression(Expression):
    parameters: List[Parameter]
    body: "Expression"


@dataclass
class ObjectLiteral(Expression):
    fields: List[Tuple[str, Expression]]


@dataclass
class Identifier(Expression):
    name: str


@dataclass
class StringLiteral(Expression):
    value: str


@dataclass
class NumberLiteral(Expression):
    value: float


@dataclass
class BooleanLiteral(Expression):
    value: bool


@dataclass
class NullLiteral(Expression):
    pass


@dataclass
class ListLiteral(Expression):
    elements: List[Expression]


@dataclass
class PropertyAccess(Expression):
    value: Expression
    attribute: str
    null_safe: bool = False


@dataclass
class IndexAccess(Expression):
    value: Expression
    index: Expression


@dataclass
class FunctionCall(Expression):
    function: Expression
    arguments: List[Expression]


@dataclass
class DefaultOp(Expression):
    left: Expression
    right: Expression


@dataclass
class IfExpression(Expression):
    condition: Expression
    when_true: Expression
    when_false: Expression


@dataclass
class MatchPattern:
    binding: Optional[str] = None
    matcher: Optional[Expression] = None
    guard: Optional[Expression] = None


@dataclass
class MatchCase:
    pattern: Optional[MatchPattern]
    expression: Expression


@dataclass
class MatchExpression(Expression):
    value: Expression
    cases: List[MatchCase]


Token = Tuple[str, Optional[str], int, int]


TOKEN_REGEX = re.compile(
    r"""
    (?P<WHITESPACE>\s+)
  | (?P<NUMBER>\d+(?:\.\d+)?)
  | (?P<STRING>"([^"\\]|\\.)*"|'([^'\\]|\\.)*')
  | (?P<DIFF>--)
  | (?P<SAFE_DOT>\?\.)
  | (?P<CONCAT>\+\+)
  | (?P<GTE>>=)
  | (?P<LTE><=)
  | (?P<EQ>==)
  | (?P<NEQ>!=)
  | (?P<ARROW>->)
  | (?P<GT>>)
  | (?P<LT><)
  | (?P<LBRACE>\{)
  | (?P<RBRACE>\})
  | (?P<LBRACKET>\[)
  | (?P<RBRACKET>\])
  | (?P<LPAREN>\()
  | (?P<RPAREN>\))
  | (?P<COLON>:)
  | (?P<COMMA>,)
  | (?P<DOT>\.)
  | (?P<PLUS>\+)
  | (?P<STAR>\*)
  | (?P<EQUAL>=)
  | (?P<IDENT>[A-Za-z_][A-Za-z0-9_]*)
  """,
    re.VERBOSE,
)


class Tokenizer:
    def __init__(self, source: str):
        self.source = source
        self.pos = 0
        self.line = 1
        self.column = 1

    def tokens(self) -> List[Token]:
        tokens: List[Token] = []
        length = len(self.source)
        while self.pos < length:
            if self.source.startswith("//", self.pos):
                comment_end = self.source.find("\n", self.pos)
                if comment_end == -1:
                    segment = self.source[self.pos :]
                    self._advance(segment)
                    self.pos = length
                else:
                    segment = self.source[self.pos : comment_end]
                    self._advance(segment)
                    self.pos = comment_end
                continue
            if self.source.startswith("/*", self.pos):
                end_index = self.source.find("*/", self.pos + 2)
                if end_index == -1:
                    raise ParseError(
                        f"Unterminated block comment at line {self.line}, column {self.column}"
                    )
                segment = self.source[self.pos : end_index + 2]
                self._advance(segment)
                self.pos = end_index + 2
                continue

            match = TOKEN_REGEX.match(self.source, self.pos)
            if not match:
                raise ParseError(
                    f"Unexpected token at line {self.line}, column {self.column}"
                )

            kind = match.lastgroup or ""
            text = match.group(kind)
            start_line = self.line
            start_column = self.column
            self._advance(text)
            self.pos = match.end()

            if kind == "WHITESPACE":
                continue

            if kind == "IDENT":
                if text == "default":
                    tokens.append(("DEFAULT", None, start_line, start_column))
                    continue
                if text in ("true", "false"):
                    tokens.append(("BOOLEAN", text, start_line, start_column))
                    continue
                if text == "null":
                    tokens.append(("NULL", None, start_line, start_column))
                    continue

            tokens.append((kind, text, start_line, start_column))

        tokens.append(("EOF", None, self.line, self.column))
        return tokens

    def _advance(self, text: str) -> None:
        for char in text:
            if char == "\n":
                self.line += 1
                self.column = 1
            else:
                self.column += 1


class Parser:
    def __init__(self, tokens: Sequence[Token]):
        self.tokens = list(tokens)
        self.index = 0

    def current(self) -> Token:
        return self.tokens[self.index]

    def advance(self) -> Token:
        token = self.current()
        if token[0] != "EOF":
            self.index += 1
        return token

    def expect(self, kind: str) -> Token:
        token = self.current()
        if token[0] != kind:
            raise ParseError(
                f"Expected {kind} but found {token[0]} at line {token[2]}, column {token[3]}"
            )
        self.advance()
        return token

    def match(self, kind: str) -> bool:
        if self.current()[0] == kind:
            self.advance()
            return True
        return False

    def parse_expression_eof(self) -> Expression:
        expr = self.parse_expression()
        if self.current()[0] != "EOF":
            token = self.current()
            raise ParseError(
                f"Unexpected tokens after expression at line {token[2]}, column {token[3]}"
            )
        return expr

    def parse_expression(self) -> Expression:
        return self.parse_if_expression()

    def parse_if_expression(self) -> Expression:
        token = self.current()
        token_type = token[0]
        token_value = token[1]
        if token_type == "IDENT" and token_value == "if":
            self.advance()
            self.expect("LPAREN")
            condition = self.parse_expression()
            self.expect("RPAREN")
            when_true = self.parse_expression()
            else_token = self.current()
            else_token_type = else_token[0]
            else_token_value = else_token[1]
            if else_token_type != "IDENT" or else_token_value != "else":
                raise ParseError(
                    f"Expected else branch in if expression at line {else_token[2]}, column {else_token[3]}"
                )
            self.advance()
            when_false = self.parse_expression()
            return IfExpression(condition=condition, when_true=when_true, when_false=when_false)
        return self.parse_default()

    def parse_default(self) -> Expression:
        expr = self.parse_comparison()
        while self.match("DEFAULT"):
            right = self.parse_comparison()
            expr = DefaultOp(left=expr, right=right)
        return expr

    def parse_comparison(self) -> Expression:
        expr = self.parse_additive()
        operator_map = {
            "EQ": "_binary_eq",
            "NEQ": "_binary_neq",
            "GT": "_binary_gt",
            "LT": "_binary_lt",
            "GTE": "_binary_gte",
            "LTE": "_binary_lte",
        }
        while True:
            token_type = self.current()[0]
            if token_type in operator_map:
                operator_name = operator_map[token_type]
                self.advance()
                right = self.parse_additive()
                expr = FunctionCall(
                    function=Identifier(name=operator_name),
                    arguments=[expr, right],
                )
            else:
                break
        return expr

    def parse_additive(self) -> Expression:
        expr = self.parse_multiplicative()
        while True:
            token_type = self.current()[0]
            if token_type == "PLUS":
                self.advance()
                right = self.parse_multiplicative()
                expr = FunctionCall(
                    function=Identifier(name="_binary_plus"),
                    arguments=[expr, right],
                )
            elif token_type == "CONCAT":
                self.advance()
                right = self.parse_multiplicative()
                expr = FunctionCall(
                    function=Identifier(name="_binary_concat"),
                    arguments=[expr, right],
                )
            elif token_type == "DIFF":
                self.advance()
                right = self.parse_multiplicative()
                expr = FunctionCall(
                    function=Identifier(name="_binary_diff"),
                    arguments=[expr, right],
                )
            else:
                break
        return expr

    def parse_multiplicative(self) -> Expression:
        expr = self.parse_postfix()
        while True:
            token_type = self.current()[0]
            if token_type == "STAR":
                self.advance()
                right = self.parse_postfix()
                expr = FunctionCall(
                    function=Identifier(name="_binary_times"),
                    arguments=[expr, right],
                )
            else:
                break
        return expr

    def parse_postfix(self) -> Expression:
        expr = self.parse_primary()
        while True:
            token = self.current()
            token_type = token[0]
            token_value = token[1]
            if token_type == "DOT":
                self.advance()
                attr_token = self.expect("IDENT")
                expr = PropertyAccess(value=expr, attribute=attr_token[1])  # type: ignore[index]
            elif token_type == "SAFE_DOT":
                self.advance()
                attr_token = self.expect("IDENT")
                expr = PropertyAccess(value=expr, attribute=attr_token[1], null_safe=True)  # type: ignore[index]
            elif token_type == "LPAREN":
                expr = self.parse_call(expr)
            elif token_type == "IDENT" and token_value not in RESERVED_INFIX_STOP:
                operator_name = token_value or ""
                self.advance()
                if operator_name == "to":
                    argument = self.parse_postfix_no_infix()
                else:
                    argument = self.parse_postfix()
                target_name = INFIX_SPECIAL.get(operator_name, operator_name)
                expr = FunctionCall(
                    function=Identifier(name=target_name),
                    arguments=[expr, argument],
                )
            elif token_type == "LBRACKET":
                self.advance()
                index_expr = self.parse_expression()
                self.expect("RBRACKET")
                expr = IndexAccess(value=expr, index=index_expr)
            elif token_type == "IDENT" and token_value == "match":
                self.advance()
                expr = self.parse_match_expression(expr)
            else:
                break
        return expr

    def parse_postfix_no_infix(self) -> Expression:
        expr = self.parse_primary()
        while True:
            token_type = self.current()[0]
            if token_type == "DOT":
                self.advance()
                attr_token = self.expect("IDENT")
                expr = PropertyAccess(value=expr, attribute=attr_token[1])  # type: ignore[index]
            elif token_type == "SAFE_DOT":
                self.advance()
                attr_token = self.expect("IDENT")
                expr = PropertyAccess(value=expr, attribute=attr_token[1], null_safe=True)  # type: ignore[index]
            elif token_type == "LPAREN":
                expr = self.parse_call(expr)
            elif token_type == "LBRACKET":
                self.advance()
                index_expr = self.parse_expression()
                self.expect("RBRACKET")
                expr = IndexAccess(value=expr, index=index_expr)
            else:
                break
        return expr

    def parse_call(self, function_expr: Expression) -> Expression:
        self.expect("LPAREN")
        args: List[Expression] = []
        if not self.match("RPAREN"):
            while True:
                args.append(self.parse_expression())
                if self.match("RPAREN"):
                    break
                self.expect("COMMA")
        return FunctionCall(function=function_expr, arguments=args)

    def parse_match_expression(self, value_expr: Expression) -> Expression:
        self.expect("LBRACE")
        cases: List[MatchCase] = []
        while not self.match("RBRACE"):
            token = self.current()
            token_type = token[0]
            token_value = token[1]
            if token_type == "IDENT" and token_value == "case":
                self.advance()
                pattern = self._parse_match_pattern()
                self.expect("ARROW")
                result_expr = self.parse_expression()
                cases.append(MatchCase(pattern=pattern, expression=result_expr))
            elif token_type == "IDENT" and token_value == "else":
                self.advance()
                self.expect("ARROW")
                result_expr = self.parse_expression()
                cases.append(MatchCase(pattern=None, expression=result_expr))
            else:
                raise ParseError(
                    f"Expected 'case' or 'else' in match expression at line {self.current()[2]}, column {self.current()[3]}"
                )
            if self.match("COMMA"):
                continue
        if not cases:
            raise ParseError("Match expression must contain at least one case")
        return MatchExpression(value=value_expr, cases=cases)

    def _parse_match_pattern(self) -> MatchPattern:
        token = self.current()
        token_type = token[0]
        token_value = token[1]
        binding: Optional[str] = None
        matcher: Optional[Expression] = None
        guard: Optional[Expression] = None
        if token_type == "IDENT" and token_value == "var":
            self.advance()
            name_token = self.expect("IDENT")
            binding = name_token[1] or ""  # type: ignore[index]
        else:
            matcher = self.parse_expression()

        if self.current()[0] == "IDENT" and self.current()[1] == "when":
            self.advance()
            guard = self.parse_expression()

        return MatchPattern(binding=binding, matcher=matcher, guard=guard)

    def _maybe_parse_lambda_expression(self) -> Optional[Expression]:
        saved_index = self.index
        try:
            return self._parse_lambda_expression_simple()
        except ParseError:
            self.index = saved_index
            try:
                return self._parse_lambda_expression_legacy()
            except ParseError:
                self.index = saved_index
                return None

    def _parse_lambda_expression_simple(self) -> Expression:
        self.expect("LPAREN")
        parameters: List[Parameter] = []
        if not self.match("RPAREN"):
            while True:
                name_token = self.expect("IDENT")
                default_expr: Optional[Expression] = None
                if self.match("EQUAL"):
                    default_expr = self.parse_expression()
                parameters.append(Parameter(name=name_token[1] or "", default=default_expr))  # type: ignore[index]
                if self.match("COMMA"):
                    continue
                self.expect("RPAREN")
                break
        self.expect("ARROW")
        body = self.parse_expression()
        return LambdaExpression(parameters=parameters, body=body)

    def _parse_lambda_expression_legacy(self) -> Expression:
        self.expect("LPAREN")
        params = self._parse_parameter_list()
        self.expect("ARROW")
        body = self.parse_expression()
        self.expect("RPAREN")
        return LambdaExpression(parameters=params, body=body)

    def _parse_parameter_list(self) -> List[Parameter]:
        self.expect("LPAREN")
        parameters: List[Parameter] = []
        if self.match("RPAREN"):
            return parameters
        while True:
            name_token = self.expect("IDENT")
            default_expr: Optional[Expression] = None
            if self.match("EQUAL"):
                default_expr = self.parse_expression()
            parameters.append(Parameter(name=name_token[1] or "", default=default_expr))  # type: ignore[index]
            if self.match("COMMA"):
                continue
            self.expect("RPAREN")
            break
        return parameters

    def parse_primary(self) -> Expression:
        token = self.current()
        token_type = token[0]
        value = token[1]
        if token_type == "LBRACE":
            return self.parse_object()
        if token_type == "LBRACKET":
            return self.parse_list()
        if token_type == "STRING":
            self.advance()
            return StringLiteral(value=_unescape_string(value or ""))
        if token_type == "NUMBER":
            self.advance()
            return NumberLiteral(value=float(value))  # type: ignore[arg-type]
        if token_type == "BOOLEAN":
            self.advance()
            return BooleanLiteral(value=(value == "true"))
        if token_type == "NULL":
            self.advance()
            return NullLiteral()
        if token_type == "IDENT":
            self.advance()
            return Identifier(name=value or "")
        if token_type == "LPAREN":
            lambda_expr = self._maybe_parse_lambda_expression()
            if lambda_expr is not None:
                return lambda_expr
            self.advance()
            expr = self.parse_expression()
            self.expect("RPAREN")
            return expr
        raise ParseError(
            f"Unexpected token {token_type} at line {token[2]}, column {token[3]}"
        )

    def parse_object(self) -> Expression:
        self.expect("LBRACE")
        fields: List[Tuple[str, Expression]] = []
        if not self.match("RBRACE"):
            while True:
                key_token = self.current()
                if key_token[0] == "STRING":
                    self.advance()
                    key = _unescape_string(key_token[1] or "")
                else:
                    key = self.expect("IDENT")[1] or ""  # type: ignore[index]
                self.expect("COLON")
                value = self.parse_expression()
                fields.append((key, value))
                if self.match("RBRACE"):
                    break
                self.expect("COMMA")
        return ObjectLiteral(fields=fields)

    def parse_list(self) -> Expression:
        self.expect("LBRACKET")
        elements: List[Expression] = []
        if not self.match("RBRACKET"):
            while True:
                elements.append(self.parse_expression())
                if self.match("RBRACKET"):
                    break
                self.expect("COMMA")
        return ListLiteral(elements=elements)


def _unescape_string(value: str) -> str:
    return bytes(value[1:-1], "utf-8").decode("unicode_escape")


def parse_script(source: str) -> Script:
    header_split = source.split("---", 1)
    if len(header_split) != 2:
        raise ParseError("Script must contain body separator '---'")
    header_source = header_split[0].strip()
    body_source = header_split[1].strip()
    header = _parse_header(header_source)
    body_expr = parse_expression_from_source(body_source)
    return Script(header=header, body=body_expr)


def parse_expression_from_source(source: str) -> Expression:
    tokenizer = Tokenizer(source)
    tokens = tokenizer.tokens()
    parser_instance = Parser(tokens)
    return parser_instance.parse_expression_eof()


def _parse_header(header_source: str) -> Header:
    version: Optional[str] = None
    output: Optional[str] = None
    imports: List[ImportDirective] = []
    variables: List[VarDeclaration] = []

    in_block_comment = False
    for idx, raw_line in enumerate(header_source.splitlines(), start=1):
        line = raw_line.strip()
        if in_block_comment:
            if "*/" in line:
                in_block_comment = False
            continue
        if line.startswith("/*"):
            if not line.endswith("*/"):
                in_block_comment = True
            continue
        if line.startswith("//"):
            continue
        if not line:
            continue
        if line.startswith("%dw"):
            parts = line.split()
            if len(parts) < 2:
                raise ParseError(f"Invalid %dw directive at header line {idx}")
            version = parts[1]
            continue
        if line.startswith("output"):
            output = line[len("output") :].strip() or None
            continue
        if line.startswith("import "):
            imports.append(ImportDirective(raw=line[len("import ") :].strip()))
            continue
        if line.startswith("var "):
            declaration_source = line[len("var ") :].strip()
            if "=" not in declaration_source:
                raise ParseError(
                    f"Invalid var declaration (missing '=') at header line {idx}"
                )
            name_part, expr_part = declaration_source.split("=", 1)
            name = name_part.strip()
            if not name:
                raise ParseError(f"Variable name cannot be empty at header line {idx}")
            expression = parse_expression_from_source(expr_part.strip())
            variables.append(VarDeclaration(name=name, expression=expression))
            continue
        raise ParseError(f"Unsupported header directive '{line}' at header line {idx}")

    if version is None:
        raise ParseError("Missing %dw directive")

    return Header(
        version=version,
        output=output,
        imports=imports,
        variables=variables,
    )

INFIX_SPECIAL = {
    "map": "_infix_map",
    "reduce": "_infix_reduce",
    "filter": "_infix_filter",
    "flatMap": "_infix_flatMap",
    "distinctBy": "_infix_distinctBy",
    "to": "_infix_to",
}

RESERVED_INFIX_STOP = {
    "else",
    "when",
    "default",
    "match",
    "case",
    "var",
}
