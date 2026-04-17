from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Any, List, Optional, Sequence, Tuple


class ParseError(ValueError):
    def __init__(self, message: str, line: Optional[int] = None, column: Optional[int] = None):
        super().__init__(message)
        self.line = line
        self.column = column


@dataclass
class VarDeclaration:
    name: str
    expression: "Expression"


@dataclass
class ImportDirective:
    raw: str


@dataclass
class FunctionDeclaration:
    name: str
    parameters: List["Parameter"]
    body: "Expression"
    return_type: Optional["TypeSpec"] = None


@dataclass
class TypeDefinition:
    name: str
    type: "TypeSpec"


@dataclass
class Header:
    version: str
    output: Optional[str]
    imports: List[ImportDirective]
    variables: List[VarDeclaration]
    functions: List[FunctionDeclaration]
    types: List[TypeDefinition]




@dataclass
class Script:
    header: Header
    body: "Expression"


class Expression:
    pass


class TypeSpec:
    pass


@dataclass
class ReferenceTypeSpec(TypeSpec):
    name: str
    generics: List["TypeSpec"]


@dataclass
class ObjectTypeSpec(TypeSpec):
    fields: List[Tuple[str, TypeSpec, bool, bool]] = field(default_factory=list)
    is_open: bool = True


@dataclass
class UnionTypeSpec(TypeSpec):
    options: List[TypeSpec]


@dataclass
class IntersectionTypeSpec(TypeSpec):
    options: List[TypeSpec]


@dataclass
class FunctionTypeSpec(TypeSpec):
    parameters: List["TypeSpec"]
    return_type: TypeSpec


@dataclass
class LiteralTypeSpec(TypeSpec):
    value: Any
    base_type: TypeSpec


@dataclass
class Parameter:
    name: str
    default: Optional["Expression"] = None
    type_annotation: Optional["TypeSpec"] = None


@dataclass
class TypeCoercion(Expression):
    expression: "Expression"
    target: TypeSpec
    options: Optional["Expression"]


@dataclass
class LambdaExpression(Expression):
    parameters: List[Parameter]
    body: "Expression"


@dataclass
class ObjectLiteral(Expression):
    fields: List[Tuple[Optional[Expression], Expression]]


@dataclass
class Identifier(Expression):
    name: str
    line: int = 0
    column: int = 0


@dataclass
class Placeholder(Expression):
    level: int
    line: int = 0
    column: int = 0


@dataclass
class StringLiteral(Expression):
    value: str


@dataclass
class TemporalLiteral(Expression):
    value: str


@dataclass
class InterpolatedString(Expression):
    parts: List[Expression]  # Mix of StringLiteral and other expressions


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
    attribute: Optional[str]
    null_safe: bool = False
    recursive: bool = False
    multi_value: bool = False
    key_value: bool = False


@dataclass
class IndexAccess(Expression):
    value: Expression
    index: Expression


@dataclass
class DynamicSelector(Expression):
    value: Expression
    selector: Expression
    mode: str


@dataclass
class FilterSelector(Expression):
    value: Expression
    predicate: Expression


@dataclass
class SelectorModifier(Expression):
    value: Expression
    mode: str


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
class DoExpression(Expression):
    header: Header
    body: "Expression"


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


Token = Tuple[str, Optional[str], int, int, int, int]


TOKEN_REGEX = re.compile(
    r"""
    (?P<WHITESPACE>\s+)
  | (?P<NUMBER>\d+(?:\.\d+)?)
  | (?P<STRING>"([^"\\]|\\.)*"|'([^'\\]|\\.)*')
  | (?P<DIFF>--)
  | (?P<DESC_DOT>\.\.)
  | (?P<SAFE_DOT>\?\.)
  | (?P<CONCAT>\+\+)
  | (?P<PIPE>\|)
  | (?P<AMP>&)
  | (?P<GTE>>=)
  | (?P<LTE><=)
  | (?P<EQ>==)
  | (?P<NEQ>!=)
  | (?P<BANG>!)
  | (?P<QUESTION>\?)
  | (?P<ARROW>->)
  | (?P<MINUS>-)
  | (?P<DIV>/)
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
  | (?P<HASH>\#)
  | (?P<CARET>\^)
  | (?P<DOT>\.)
  | (?P<PLUS>\+)
  | (?P<STAR>\*)
  | (?P<AT>@)
  | (?P<EQUAL>=)
  | (?P<DOLLAR>\$\$?)
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
                        f"Unterminated block comment at line {self.line}, column {self.column}",
                        self.line,
                        self.column,
                    )
                segment = self.source[self.pos : end_index + 2]
                self._advance(segment)
                self.pos = end_index + 2
                continue

            match = TOKEN_REGEX.match(self.source, self.pos)
            if not match:
                raise ParseError(
                    f"Unexpected token at line {self.line}, column {self.column}",
                    self.line,
                    self.column,
                )

            kind = match.lastgroup or ""
            text = match.group(kind)
            start_line = self.line
            start_column = self.column
            start_offset = self.pos
            self._advance(text)
            self.pos = match.end()
            end_offset = self.pos

            if kind == "WHITESPACE":
                continue

            if kind == "IDENT":
                if text == "default":
                    tokens.append(("DEFAULT", None, start_line, start_column, start_offset, end_offset))
                    continue
                if text in ("true", "false"):
                    tokens.append(("BOOLEAN", text, start_line, start_column, start_offset, end_offset))
                    continue
                if text == "null":
                    tokens.append(("NULL", None, start_line, start_column, start_offset, end_offset))
                    continue

            tokens.append((kind, text, start_line, start_column, start_offset, end_offset))

        tokens.append(("EOF", None, self.line, self.column, self.pos, self.pos))
        return tokens

    def _advance(self, text: str) -> None:
        for char in text:
            if char == "\n":
                self.line += 1
                self.column = 1
            else:
                self.column += 1


class Parser:
    def __init__(self, tokens: Sequence[Token], source: str):
        self.tokens = list(tokens)
        self.index = 0
        self.source = source

    def current(self) -> Token:
        return self.tokens[self.index]

    def advance(self) -> Token:
        token = self.current()
        if token[0] != "EOF":
            self.index += 1
        return token

    def at_end(self) -> bool:
        return self.current()[0] == "EOF"

    def peek(self, offset: int) -> Token:
        index = self.index + offset
        if index < 0:
            index = 0
        if index >= len(self.tokens):
            return self.tokens[-1]
        return self.tokens[index]

    def expect(self, kind: str) -> Token:
        token = self.current()
        if token[0] != kind:
            raise ParseError(
                f"Expected {kind} but found {token[0]} at line {token[2]}, column {token[3]}",
                token[2],
                token[3],
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
                f"Unexpected tokens after expression at line {token[2]}, column {token[3]}",
                token[2],
                token[3],
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
                    f"Expected else branch in if expression at line {else_token[2]}, column {else_token[3]}",
                    else_token[2],
                    else_token[3],
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
                plus_token = self.current()
                self.advance()
                right = self.parse_multiplicative()
                expr = FunctionCall(
                    function=Identifier(
                        name="_binary_plus",
                        line=plus_token[2],
                        column=plus_token[3],
                    ),
                    arguments=[expr, right],
                )
            elif token_type == "CONCAT":
                self.advance()
                right = self.parse_multiplicative()
                expr = FunctionCall(
                    function=Identifier(name="_binary_concat"),
                    arguments=[expr, right],
                )
            elif token_type == "MINUS":
                self.advance()
                right = self.parse_multiplicative()
                expr = FunctionCall(
                    function=Identifier(name="_binary_minus"),
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
            elif token_type == "DIV":
                self.advance()
                right = self.parse_postfix()
                expr = FunctionCall(
                    function=Identifier(name="_binary_divide"),
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
            if token_type == "IDENT" and token_value == "as":
                self.advance()
                target_type = self._parse_type_spec(consume_metadata=False)
                options_expr: Optional[Expression] = None
                if self.current()[0] == "LBRACE":
                    options_expr = self.parse_expression()
                expr = TypeCoercion(expression=expr, target=target_type, options=options_expr)
                continue
            if token_type == "DOT":
                self.advance()
                expr = self._parse_dot_selector(expr)
            elif token_type == "DESC_DOT":
                self.advance()
                expr = self._parse_descendant_selector(expr)
            elif token_type == "SAFE_DOT":
                self.advance()
                expr = PropertyAccess(
                    value=expr,
                    attribute=self._parse_property_attribute(allow_special=False),
                    null_safe=True,
                )
            elif token_type == "QUESTION":
                self.advance()
                expr = SelectorModifier(value=expr, mode="present")
            elif token_type == "BANG":
                self.advance()
                expr = SelectorModifier(value=expr, mode="assert")
            elif token_type == "LPAREN":
                expr = self.parse_call(expr)
            elif token_type == "IDENT" and token_value not in RESERVED_INFIX_STOP:
                operator_name = token_value or ""
                self.advance()
                argument = self.parse_postfix_no_infix()
                target_name = INFIX_SPECIAL.get(operator_name, operator_name)
                expr = FunctionCall(
                    function=Identifier(
                        name=target_name,
                        line=token[2],
                        column=token[3],
                    ),
                    arguments=[expr, argument],
                )
            elif token_type == "LBRACKET":
                expr = self._parse_bracket_selector(expr)
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
                expr = self._parse_dot_selector(expr)
            elif token_type == "DESC_DOT":
                self.advance()
                expr = self._parse_descendant_selector(expr)
            elif token_type == "SAFE_DOT":
                self.advance()
                expr = PropertyAccess(
                    value=expr,
                    attribute=self._parse_property_attribute(allow_special=False),
                    null_safe=True,
                )
            elif token_type == "QUESTION":
                self.advance()
                expr = SelectorModifier(value=expr, mode="present")
            elif token_type == "BANG":
                self.advance()
                expr = SelectorModifier(value=expr, mode="assert")
            elif token_type == "LPAREN":
                expr = self.parse_call(expr)
            elif token_type == "LBRACKET":
                expr = self._parse_bracket_selector(expr)
            else:
                break
        return expr

    def _parse_dot_selector(self, expr: Expression) -> Expression:
        token_type = self.current()[0]
        if token_type == "STAR":
            self.advance()
            return PropertyAccess(
                value=expr,
                attribute=self._parse_property_attribute(allow_special=False),
                multi_value=True,
            )
        if token_type == "AMP":
            self.advance()
            return PropertyAccess(
                value=expr,
                attribute=self._parse_property_attribute(allow_special=False),
                key_value=True,
            )
        if token_type == "AT":
            self.advance()
            if self.current()[0] == "DOT":
                self.advance()
            return PropertyAccess(
                value=expr,
                attribute=f"@{self._parse_property_attribute(allow_special=False)}",
            )
        return PropertyAccess(
            value=expr,
            attribute=self._parse_property_attribute(),
        )

    def _parse_descendant_selector(self, expr: Expression) -> Expression:
        token_type = self.current()[0]
        if token_type in {"EOF", "RPAREN", "RBRACKET", "RBRACE", "COMMA"}:
            return PropertyAccess(value=expr, attribute=None, recursive=True)
        if token_type == "STAR":
            self.advance()
            if self.current()[0] in {"EOF", "RPAREN", "RBRACKET", "RBRACE", "COMMA"}:
                return PropertyAccess(value=expr, attribute=None, recursive=True, multi_value=True)
            return PropertyAccess(
                value=expr,
                attribute=self._parse_property_attribute(allow_special=False),
                recursive=True,
                multi_value=True,
            )
        if token_type == "AMP":
            self.advance()
            return PropertyAccess(
                value=expr,
                attribute=self._parse_property_attribute(allow_special=False),
                recursive=True,
                key_value=True,
            )
        return PropertyAccess(
            value=expr,
            attribute=self._parse_property_attribute(allow_special=False),
            recursive=True,
        )

    def _parse_property_attribute(self, allow_special: bool = True) -> str:
        attr_token = self.current()
        if allow_special and attr_token[0] == "STAR":
            self.advance()
            name_token = self.expect("IDENT")
            return f"*{name_token[1]}"  # type: ignore[index]
        if allow_special and attr_token[0] == "AT":
            self.advance()
            name_token = self.expect("IDENT")
            return f"@{name_token[1]}"  # type: ignore[index]
        if attr_token[0] == "STRING":
            self.advance()
            return _unescape_string(attr_token[1] or "")
        ident_token = self.expect("IDENT")
        return ident_token[1] or ""  # type: ignore[index]

    def _parse_bracket_selector(self, expr: Expression) -> Expression:
        self.expect("LBRACKET")
        token_type = self.current()[0]
        if token_type == "QUESTION" and self.peek(1)[0] == "LPAREN":
            self.advance()
            self.expect("LPAREN")
            predicate = self.parse_expression()
            self.expect("RPAREN")
            self.expect("RBRACKET")
            return FilterSelector(value=expr, predicate=predicate)
        if token_type in {"STAR", "AT", "AMP"} and self.peek(1)[0] == "LPAREN":
            marker = token_type
            self.advance()
            self.expect("LPAREN")
            selector = self.parse_expression()
            self.expect("RPAREN")
            self.expect("RBRACKET")
            mode_map = {
                "STAR": "multi",
                "AT": "attribute",
                "AMP": "key_value",
            }
            return DynamicSelector(value=expr, selector=selector, mode=mode_map[marker])
        index_expr = self.parse_expression()
        self.expect("RBRACKET")
        return IndexAccess(value=expr, index=index_expr)

    def _parse_type_spec(self, consume_metadata: bool = True) -> TypeSpec:
        def parse_primary() -> TypeSpec:
            token = self.current()
            ttype = token[0]
            tvalue = token[1]
            if ttype == "IDENT":
                self.advance()
                name = tvalue or ""
                generics: List[TypeSpec] = []
                if self.current()[0] == "LT":
                    self.advance()
                    while True:
                        generics.append(self._parse_type_spec(consume_metadata=consume_metadata))
                        if self.current()[0] == "COMMA":
                            self.advance()
                            continue
                        self.expect("GT")
                        break
                ref = ReferenceTypeSpec(name=name, generics=generics)
                # Skip metadata blocks after type references (e.g., String { format: \"...\" })
                if consume_metadata and self.current()[0] == "LBRACE":
                    depth = 0
                    while not self.at_end():
                        curr = self.current()[0]
                        if curr == "LBRACE":
                            depth += 1
                        elif curr == "RBRACE":
                            depth -= 1
                            if depth == 0:
                                self.advance()
                                break
                        self.advance()
                    if depth != 0:
                        raise ParseError("Unterminated type annotation block", token[2], token[3])
                return ref
            raise ParseError(f"Expected type name or identifier, found {ttype}", token[2], token[3])

        left = parse_primary()
        while self.current()[0] in {"PIPE", "AMP"}:
            op = self.current()[0]
            self.advance()
            right = parse_primary()
            if op == "PIPE":
                if isinstance(left, UnionTypeSpec):
                    left.options.append(right)
                else:
                    left = UnionTypeSpec(options=[left, right])
            else:
                if isinstance(left, IntersectionTypeSpec):
                    left.options.append(right)
                else:
                    left = IntersectionTypeSpec(options=[left, right])
        return left

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
                current = self.current()
                raise ParseError(
                    f"Expected 'case' or 'else' in match expression at line {current[2]}, column {current[3]}",
                    current[2],
                    current[3],
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
        if token_type == "BANG":
            self.advance()
            operand = self.parse_postfix_no_infix()
            return FunctionCall(
                function=Identifier(
                    name="_unary_not",
                    line=token[2],
                    column=token[3],
                ),
                arguments=[operand],
            )
        if token_type == "MINUS":
            self.advance()
            operand = self.parse_postfix_no_infix()
            return FunctionCall(
                function=Identifier(
                    name="_binary_minus",
                    line=token[2],
                    column=token[3],
                ),
                arguments=[NumberLiteral(value=0.0), operand],
            )
        if token_type == "IDENT" and value == "do":
            return self.parse_do_expression()
        if token_type == "IDENT" and value == "match":
            self.advance()
            value_expr = self.parse_expression()
            if self.current()[0] != "LBRACE":
                raise ParseError("Expected '{' after match expression")
            return self.parse_match_expression(value_expr)
        if token_type == "LBRACE":
            return self.parse_object()
        if token_type == "LBRACKET":
            return self.parse_list()
        if token_type == "STRING":
            self.advance()
            unescaped = _unescape_string(value or "")
            # Check for string interpolation
            if "$(" in unescaped:
                return self._parse_interpolated_string(unescaped)
            return StringLiteral(value=unescaped)
        if token_type == "PIPE":
            return self._parse_temporal_literal()
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
            return Identifier(name=value or "", line=token[2], column=token[3])
        if token_type == "DOLLAR":
            self.advance()
            placeholder_text = value or ""
            return Placeholder(level=len(placeholder_text), line=token[2], column=token[3])
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

    def _parse_temporal_literal(self) -> Expression:
        opening = self.expect("PIPE")
        closing_index = self.index
        while closing_index < len(self.tokens):
            token = self.tokens[closing_index]
            if token[0] == "PIPE":
                break
            if token[0] == "EOF":
                raise ParseError(
                    "Unterminated temporal literal",
                    opening[2],
                    opening[3],
                )
            closing_index += 1

        if closing_index >= len(self.tokens) or self.tokens[closing_index][0] != "PIPE":
            raise ParseError(
                "Unterminated temporal literal",
                opening[2],
                opening[3],
            )

        closing = self.tokens[closing_index]
        inner = self.source[opening[5] : closing[4]]
        self.index = closing_index + 1
        return TemporalLiteral(value=inner.strip())

    def parse_do_expression(self) -> Expression:
        do_token = self.current()
        self.advance()
        lbrace_token = self.expect("LBRACE")
        content_start = lbrace_token[4] + 1

        depth = 1
        scan_index = self.index
        closing_token: Optional[Token] = None
        while scan_index < len(self.tokens):
            token = self.tokens[scan_index]
            kind = token[0]
            if kind == "LBRACE":
                depth += 1
            elif kind == "RBRACE":
                depth -= 1
                if depth == 0:
                    closing_token = token
                    break
            scan_index += 1

        if closing_token is None:
            raise ParseError(
                f"Unterminated do block at line {do_token[2]}, column {do_token[3]}",
                do_token[2],
                do_token[3],
            )

        content_end = closing_token[4]
        inner_source = self.source[content_start:content_end]
        self.index = scan_index + 1
        return _parse_do_block_content(inner_source)

    def parse_object(self) -> Expression:
        self.expect("LBRACE")
        fields: List[Tuple[Optional[Expression], Expression]] = []
        if not self.match("RBRACE"):
            while True:
                key_token = self.current()
                if key_token[0] == "LPAREN":
                    self.advance()
                    inner_expr = self.parse_expression()
                    self.expect("RPAREN")
                    if self.match("COLON"):
                        key_expr = inner_expr
                        value = self.parse_expression()
                        fields.append((key_expr, value))
                    else:
                        fields.append((None, inner_expr))
                elif key_token[0] == "STRING":
                    self.advance()
                    unescaped = _unescape_string(key_token[1] or "")
                    if "$(" in unescaped:
                        key_expr = self._parse_interpolated_string(unescaped)
                    else:
                        key_expr = StringLiteral(value=unescaped)
                    self.expect("COLON")
                    value = self.parse_expression()
                    fields.append((key_expr, value))
                else:
                    ident = self.expect("IDENT")
                    key_expr = StringLiteral(value=ident[1] or "")
                    self.expect("COLON")
                    value = self.parse_expression()
                    fields.append((key_expr, value))
                if self.match("RBRACE"):
                    break
                self.expect("COMMA")
                if self.match("RBRACE"):
                    break
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

    def _parse_interpolated_string(self, content: str) -> Expression:
        """Parse a string with $(expression) interpolations."""
        parts: List[Expression] = []
        pos = 0
        
        while pos < len(content):
            # Find the next interpolation
            start = content.find("$(", pos)
            
            if start == -1:
                # No more interpolations, add remaining string
                if pos < len(content):
                    parts.append(StringLiteral(value=content[pos:]))
                break
            
            # Add the string literal before the interpolation
            if start > pos:
                parts.append(StringLiteral(value=content[pos:start]))
            
            # Find the matching closing parenthesis
            paren_depth = 1
            idx = start + 2  # Start after "$("
            while idx < len(content) and paren_depth > 0:
                if content[idx] == '(':
                    paren_depth += 1
                elif content[idx] == ')':
                    paren_depth -= 1
                idx += 1
            
            if paren_depth != 0:
                raise ParseError("Unclosed interpolation expression in string")
            
            # Parse the expression inside $(...)
            expr_source = content[start + 2:idx - 1]
            expr = parse_expression_from_source(expr_source)
            parts.append(expr)
            
            pos = idx
        
        # If no parts, return an empty string
        if not parts:
            return StringLiteral(value="")
        
        # If only one part and it's a string literal, return it directly
        if len(parts) == 1 and isinstance(parts[0], StringLiteral):
            return parts[0]
        
        return InterpolatedString(parts=parts)


def _unescape_string(value: str) -> str:
    return bytes(value[1:-1], "utf-8").decode("unicode_escape")


def parse_script(source: str) -> Script:
    stripped = source.strip()
    delimiter_line = _find_top_level_script_delimiter_line(stripped)
    if delimiter_line is None:
        if not stripped:
            raise ParseError("Script body cannot be empty")
        header = Header(
            version="2.0",
            output=None,
            imports=[],
            variables=[],
            functions=[],
            types=[],
        )
        body_expr = parse_expression_from_source(stripped)
        return Script(header=header, body=body_expr)
    lines = stripped.splitlines()
    header_source = "\n".join(lines[:delimiter_line])
    body_source = "\n".join(lines[delimiter_line + 1 :])
    header = _parse_header(header_source.strip())
    body_expr = parse_expression_from_source(body_source.strip())
    return Script(header=header, body=body_expr)


def _find_top_level_script_delimiter_line(source: str) -> Optional[int]:
    lines = source.splitlines()
    curly = 0
    square = 0
    paren = 0
    quote: Optional[str] = None
    escaped = False
    in_block_comment = False

    def update_balances(line: str) -> None:
        nonlocal curly, square, paren, quote, escaped, in_block_comment
        i = 0
        while i < len(line):
            ch = line[i]
            nxt = line[i + 1] if i + 1 < len(line) else ""

            if in_block_comment:
                if ch == "*" and nxt == "/":
                    in_block_comment = False
                    i += 2
                    continue
                i += 1
                continue

            if quote is not None:
                if escaped:
                    escaped = False
                    i += 1
                    continue
                if ch == "\\":
                    escaped = True
                    i += 1
                    continue
                if ch == quote:
                    quote = None
                i += 1
                continue

            if ch == "/" and nxt == "*":
                in_block_comment = True
                i += 2
                continue
            if ch == "/" and nxt == "/":
                break

            if ch in ("'", '"'):
                quote = ch
                i += 1
                continue
            if ch == "{":
                curly += 1
            elif ch == "}":
                curly -= 1
            elif ch == "[":
                square += 1
            elif ch == "]":
                square -= 1
            elif ch == "(":
                paren += 1
            elif ch == ")":
                paren -= 1
            i += 1

    for index, line in enumerate(lines):
        if (
            line.strip() == "---"
            and curly == 0
            and square == 0
            and paren == 0
            and quote is None
            and not in_block_comment
        ):
            return index
        update_balances(line)
    return None


def parse_expression_from_source(source: str) -> Expression:
    tokenizer = Tokenizer(source)
    tokens = tokenizer.tokens()
    parser_instance = Parser(tokens, source)
    return parser_instance.parse_expression_eof()


def _parse_header(header_source: str) -> Header:
    version: Optional[str] = None
    output: Optional[str] = None
    imports: List[ImportDirective] = []
    variables: List[VarDeclaration] = []
    functions: List[FunctionDeclaration] = []
    types: List[TypeDefinition] = []

    lines = header_source.splitlines()
    num_lines = len(lines)

    def _compute_delimiter_balance(text: str) -> Tuple[int, int, int]:
        curly = 0
        square = 0
        paren = 0
        quote: Optional[str] = None
        escaped = False
        in_block_comment = False
        i = 0
        while i < len(text):
            ch = text[i]
            nxt = text[i + 1] if i + 1 < len(text) else ""

            if in_block_comment:
                if ch == "*" and nxt == "/":
                    in_block_comment = False
                    i += 2
                    continue
                i += 1
                continue

            if quote is not None:
                if escaped:
                    escaped = False
                    i += 1
                    continue
                if ch == "\\":
                    escaped = True
                    i += 1
                    continue
                if ch == quote:
                    quote = None
                i += 1
                continue

            if ch == "/" and nxt == "*":
                in_block_comment = True
                i += 2
                continue
            if ch == "/" and nxt == "/":
                # Ignore until end of current line.
                newline_idx = text.find("\n", i)
                if newline_idx == -1:
                    break
                i = newline_idx + 1
                continue

            if ch in ("'", '"'):
                quote = ch
                i += 1
                continue
            if ch == "{":
                curly += 1
            elif ch == "}":
                curly -= 1
            elif ch == "[":
                square += 1
            elif ch == "]":
                square -= 1
            elif ch == "(":
                paren += 1
            elif ch == ")":
                paren -= 1
            i += 1
        return curly, square, paren

    def _needs_multiline_expression(text: str) -> bool:
        curly, square, paren = _compute_delimiter_balance(text)
        return curly > 0 or square > 0 or paren > 0

    def _line_indent(raw_line: str) -> int:
        return len(raw_line) - len(raw_line.lstrip(" \t"))

    def _is_header_directive_line(raw_line: str) -> bool:
        stripped = raw_line.strip()
        return (
            stripped.startswith("%dw")
            or stripped.startswith("output")
            or stripped.startswith("import ")
            or stripped.startswith("type ")
            or stripped.startswith("var ")
            or stripped.startswith("fun ")
        )

    def _next_significant_line_index(start_idx: int) -> Optional[int]:
        current_idx = start_idx
        while current_idx < num_lines:
            stripped = lines[current_idx].strip()
            if stripped and not stripped.startswith("//"):
                return current_idx
            current_idx += 1
        return None

    def _expression_parse_status(source: str) -> str:
        stripped = source.strip()
        if not stripped:
            return "empty"
        if _needs_multiline_expression(stripped):
            return "incomplete"
        try:
            parse_expression_from_source(stripped)
        except ParseError as exc:
            message = str(exc)
            incomplete_suffixes = (
                "->",
                "=",
                "(",
                "[",
                "{",
                ",",
                "++",
                "default",
                "and",
                "or",
                "match",
            )
            if (
                "Expected else branch in if expression" in message
                or "but found EOF" in message
                or "Unexpected token EOF" in message
                or stripped.endswith(incomplete_suffixes)
            ):
                return "incomplete"
            return "invalid"
        return "complete"

    def _read_multiline_expression(start_line_idx: int, initial_expression: str) -> Tuple[str, int]:
        expression = initial_expression
        total_lines_consumed = 1
        current_idx = start_line_idx + 1
        while current_idx < num_lines and _needs_multiline_expression(expression):
            expression += "\n" + lines[current_idx]
            total_lines_consumed += 1
            current_idx += 1
        if _needs_multiline_expression(expression):
            raise ParseError("Unterminated multi-line expression in header")
        return expression, total_lines_consumed

    def _read_header_expression_block(
        start_line_idx: int,
        initial_expression: str,
    ) -> Tuple[str, int]:
        declaration_indent = _line_indent(lines[start_line_idx])
        expression_parts: List[str] = [initial_expression] if initial_expression else []
        current_idx = start_line_idx + 1

        while True:
            expression_source = "\n".join(expression_parts).strip()
            status = _expression_parse_status(expression_source)
            next_idx = _next_significant_line_index(current_idx)

            if next_idx is None:
                break

            next_line = lines[next_idx]
            next_indent = _line_indent(next_line)
            next_chunk = "\n".join(lines[current_idx : next_idx + 1])
            combined_source = (
                f"{expression_source}\n{next_chunk}".strip()
                if expression_source
                else next_chunk.strip()
            )
            combined_status = _expression_parse_status(combined_source)
            next_is_directive = _is_header_directive_line(next_line)

            should_continue = False
            if status == "empty":
                should_continue = not next_is_directive
            elif status == "incomplete":
                should_continue = True
            elif next_is_directive:
                should_continue = False
            elif next_indent > declaration_indent:
                should_continue = True
            elif combined_status in {"complete", "incomplete"}:
                should_continue = True

            if not should_continue:
                break

            expression_parts.append(next_chunk)
            current_idx = next_idx + 1

        expression_source = "\n".join(expression_parts).strip()
        return expression_source, current_idx - start_line_idx

    def _read_multiline_definition(start_line_idx: int, start_col: int, start_char: str, end_char: str) -> Tuple[str, int]:
        definition_parts = [lines[start_line_idx][start_col:]]
        balance = definition_parts[0].count(start_char) - definition_parts[0].count(end_char)
        total_lines_consumed = 1
        current_idx = start_line_idx + 1
        while current_idx < num_lines and balance > 0:
            line_to_add = lines[current_idx]
            definition_parts.append(line_to_add)
            balance += line_to_add.count(start_char)
            balance -= line_to_add.count(end_char)
            current_idx += 1
            total_lines_consumed += 1
            if balance == 0:
                break
        if balance > 0:
            raise ParseError("Unterminated multi-line definition in header")
        return "\n".join(definition_parts), total_lines_consumed

    in_block_comment = False
    idx = 0
    while idx < num_lines:
        raw_line = lines[idx]
        line = raw_line.strip()
        line_number = idx + 1

        if in_block_comment:
            if "*/" in line:
                in_block_comment = False
            idx += 1
            continue
        if line.startswith("/*"):
            if not line.endswith("*/"):
                in_block_comment = True
            idx += 1
            continue
        if line.startswith("//"):
            idx += 1
            continue
        if not line:
            idx += 1
            continue
        if line.startswith("%dw"):
            parts = line.split()
            if len(parts) < 2:
                raise ParseError(f"Invalid %dw directive at header line {line_number}", line_number, 1)
            version = parts[1]
            idx += 1
            continue
        if line.startswith("output"):
            output = line[len("output") :].strip() or None
            idx += 1
            continue
        if line.startswith("import "):
            imports.append(ImportDirective(raw=line[len("import ") :].strip()))
            idx += 1
            continue
        if line.startswith("type "):
            remaining_line_from_type_keyword = line[len("type ") :]
            equals_pos_in_remaining = _find_top_level_char(remaining_line_from_type_keyword, "=")
            if equals_pos_in_remaining == -1:
                raise ParseError(f"Invalid type definition (missing top-level '=') at header line {line_number}", line_number, 1)

            name_part = remaining_line_from_type_keyword[:equals_pos_in_remaining].strip()
            type_definition_start_str = remaining_line_from_type_keyword[equals_pos_in_remaining + 1 :].strip()
            if not name_part:
                raise ParseError(f"Type name cannot be empty at header line {line_number}", line_number, 1)

            full_type_spec_string = type_definition_start_str
            lines_to_advance = 1

            start_char_match = re.search(r"[{([<]", type_definition_start_str)
            if start_char_match:
                start_char_val = start_char_match.group(0)
                end_char_val = {"{": "}", "(": ")", "[": "]", "<": ">"}[start_char_val]
                first_line_balance = type_definition_start_str.count(start_char_val) - type_definition_start_str.count(end_char_val)
                if first_line_balance > 0 or (
                    start_char_match.start() != -1 and type_definition_start_str.find(end_char_val, start_char_match.start()) == -1
                ):
                    search_offset = raw_line.find(type_definition_start_str)
                    if search_offset == -1:
                        search_offset = raw_line.find(start_char_val)
                    start_char_col_in_raw_line = raw_line.find(start_char_val, search_offset)
                    if start_char_col_in_raw_line != -1:
                        multiline_content_from_char, total_lines_for_this_type_def = _read_multiline_definition(
                            idx, start_char_col_in_raw_line, start_char_val, end_char_val
                        )
                        prefix_before_start_char = type_definition_start_str[: start_char_match.start()]
                        full_type_spec_string = prefix_before_start_char + multiline_content_from_char
                        lines_to_advance = total_lines_for_this_type_def

            try:
                type_spec = _parse_type_spec_string(full_type_spec_string.strip())
            except ParseError as e:
                raise ParseError(f"Invalid type expression for '{name_part}' at header line {line_number}: {e}", line_number, 1) from e

            types.append(TypeDefinition(name=name_part, type=type_spec))
            idx += lines_to_advance
            continue
        if line.startswith("var "):
            declaration_source = line[len("var ") :].strip()
            if "=" not in declaration_source:
                raise ParseError(f"Invalid var declaration (missing '=') at header line {line_number}", line_number, 1)
            name_part, expr_part = declaration_source.split("=", 1)
            name = name_part.strip()
            if not name:
                raise ParseError(f"Variable name cannot be empty at header line {line_number}", line_number, 1)
            expr_source = expr_part.strip()
            expr_source, lines_to_advance = _read_header_expression_block(idx, expr_source)
            if not expr_source:
                raise ParseError(f"Missing variable body at header line {line_number}", line_number, 1)
            expression = parse_expression_from_source(expr_source.strip())
            variables.append(VarDeclaration(name=name, expression=expression))
            idx += lines_to_advance
            continue
        if line.startswith("fun "):
            function_source = line[len("fun ") :].strip()
            body_delimiter = _find_top_level_char(function_source, "=")
            if body_delimiter != -1:
                signature_part = function_source[:body_delimiter]
                body_part = function_source[body_delimiter + 1 :]
                body_source, lines_to_advance = _read_header_expression_block(idx, body_part.strip())
                function_source = f"{signature_part.strip()} = {body_source.strip()}"
            else:
                lines_to_advance = 1
            function = _parse_header_function(function_source, line_number)
            functions.append(function)
            idx += lines_to_advance
            continue
        raise ParseError(f"Unsupported header directive '{line}' at header line {line_number}", line_number, 1)

    if version is None:
        raise ParseError("Missing %dw directive")

    return Header(
        version=version,
        output=output,
        imports=imports,
        variables=variables,
        functions=functions,
        types=types,
    )


def _parse_header_function(source: str, line_no: int) -> FunctionDeclaration:
    body_delimiter = _find_top_level_char(source, "=")
    if body_delimiter == -1:
        raise ParseError(f"Invalid function declaration at header line {line_no}", line_no, 1)
    signature_part = source[:body_delimiter]
    body_part = source[body_delimiter + 1 :]
    signature_part = signature_part.strip()
    body_part = body_part.strip()
    if not body_part:
        raise ParseError(f"Missing function body at header line {line_no}", line_no, 1)
    match = re.match(
        r"^([A-Za-z_][A-Za-z0-9_]*)(?:<[^>]*>)?\s*\((.*)\)\s*(?::\s*(.+))?$",
        signature_part,
    )
    if not match:
        raise ParseError(f"Invalid function signature at header line {line_no}", line_no, 1)
    name = match.group(1)
    params_source = match.group(2)
    return_type_source = match.group(3)
    parameters = _parse_header_function_parameters(params_source)
    body_expr = parse_expression_from_source(body_part)
    return_type = (
        _parse_type_spec_string(return_type_source) if return_type_source else None
    )
    return FunctionDeclaration(
        name=name,
        parameters=parameters,
        body=body_expr,
        return_type=return_type,
    )


def _parse_do_block_content(content_source: str) -> DoExpression:
    inner = content_source.strip()
    if not inner:
        raise ParseError("Do block cannot be empty")
    wrapped_script = "%dw 2.0\n" + inner
    script = parse_script(wrapped_script)
    return DoExpression(header=script.header, body=script.body)


def _parse_header_function_parameters(params_source: str) -> List[Parameter]:
    params_source = params_source.strip()
    if not params_source:
        return []
    parts = _split_top_level(params_source, ",")
    parameters: List[Parameter] = []
    for part in parts:
        segment = part.strip()
        if not segment:
            continue
        name_section = segment
        default_expr: Optional[Expression] = None
        type_annotation: Optional[TypeSpec] = None
        equals_split = _split_top_level(segment, "=", maxsplit=1)
        if len(equals_split) == 2:
            name_section = equals_split[0].strip()
            default_source = equals_split[1].strip()
            if not default_source:
                raise ParseError("Default parameter expression cannot be empty")
            default_expr = parse_expression_from_source(default_source)
        if ":" in name_section:
            name_part, type_part = name_section.split(":", 1)
            name_section = name_part.strip()
            type_source = type_part.strip()
            if type_source:
                type_annotation = _parse_type_spec_string(type_source)
        name = name_section.strip()
        if not name:
            raise ParseError("Function parameter name cannot be empty")
        parameters.append(Parameter(name=name, default=default_expr, type_annotation=type_annotation))
    return parameters


def _split_top_level(source: str, delimiter: str, *, maxsplit: int = -1) -> List[str]:
    if delimiter not in source:
        return [source]
    result: List[str] = []
    current: List[str] = []
    depth = 0
    splits_done = 0
    for char in source:
        if char in "({[":
            depth += 1
        elif char in ")}]":
            if depth > 0:
                depth -= 1
        if char == delimiter and depth == 0 and (maxsplit < 0 or splits_done < maxsplit):
            result.append("".join(current))
            current = []
            splits_done += 1
            continue
        current.append(char)
    result.append("".join(current))
    return result


def _find_top_level_char(source: str, char_to_find: str, *, start_index: int = 0) -> int:
    depth = 0
    for i, char in enumerate(source[start_index:], start=start_index):
        if char in "({[":
            depth += 1
        elif char in ")]}":
            if depth > 0:
                depth -= 1
        if char == char_to_find and depth == 0:
            return i
    return -1


def _parse_type_spec_string(source: str) -> TypeSpec:
    parser = _TypeSpecParser(source)
    type_spec = parser.parse_type_spec()
    parser.skip_whitespace()
    if not parser.at_end():
        raise ParseError(f"Invalid type specification '{source}'")
    return type_spec


class _TypeSpecParser:
    def __init__(self, source: str):
        self.source = source
        self.index = 0

    def at_end(self) -> bool:
        return self.index >= len(self.source)

    def current(self) -> Optional[str]:
        if self.at_end():
            return None
        return self.source[self.index]

    def peek(self, offset: int) -> Optional[str]:
        idx = self.index + offset
        if idx >= len(self.source):
            return None
        return self.source[idx]

    def advance(self) -> Optional[str]:
        if self.at_end():
            return None
        char = self.source[self.index]
        self.index += 1
        return char

    def skip_whitespace(self) -> None:
        while not self.at_end() and self.source[self.index].isspace():
            self.index += 1

    def parse_identifier(self) -> str:
        self.skip_whitespace()
        start = self.index
        while not self.at_end() and (self.source[self.index].isalnum() or self.source[self.index] == "_"):
            self.index += 1
        if start == self.index:
            raise ParseError("Expected type name or identifier")
        return self.source[start:self.index]

    def parse_type_spec(self) -> TypeSpec:
        left_type = self._parse_primary_type()
        self.skip_whitespace()
        while not self.at_end():
            # Stop if we are about to hit a closed-object terminator
            if self.current() == "}" or (self.current() == "|" and self.peek(1) == "}"):
                break
            if self.current() == "|":
                self.advance()
                right_type = self._parse_primary_type()
                if isinstance(left_type, UnionTypeSpec):
                    left_type.options.append(right_type)
                else:
                    left_type = UnionTypeSpec(options=[left_type, right_type])
            elif self.current() == "&":
                self.advance()
                right_type = self._parse_primary_type()
                if isinstance(left_type, IntersectionTypeSpec):
                    left_type.options.append(right_type)
                else:
                    left_type = IntersectionTypeSpec(options=[left_type, right_type])
            else:
                break
            self.skip_whitespace()
        return left_type

    def _parse_primary_type(self) -> TypeSpec:
        self.skip_whitespace()
        current_char = self.current()

        if current_char == "{":
            return self.parse_object_type()
        elif current_char == "(":
            return self.parse_function_type_or_grouped_type()
        elif current_char == '"':
            return self.parse_string_literal_type()
        elif current_char and (current_char.isdigit() or current_char == "-"):
            return self.parse_number_literal_type()
        elif self.source[self.index :].startswith("true") or self.source[self.index :].startswith("false"):
            return self.parse_boolean_literal_type()
        else:
            return self.parse_reference_type()

    def parse_reference_type(self) -> ReferenceTypeSpec:
        name = self.parse_identifier()
        generics: List[TypeSpec] = []
        self.skip_whitespace()
        if self.current() == "<":
            self.advance()
            while True:
                generics.append(self.parse_type_spec())
                self.skip_whitespace()
                if self.current() == ",":
                    self.advance()
                    continue
                if self.current() == ">":
                    self.advance()
                    break
                raise ParseError("Unterminated generic specification")
        # Ignore metadata/annotations after the type, e.g. String { format: \"...\" }
        self.skip_whitespace()
        if self.current() == "{":
            depth = 0
            while not self.at_end():
                ch = self.current()
                if ch == "{":
                    depth += 1
                elif ch == "}":
                    depth -= 1
                    if depth == 0:
                        self.advance()
                        break
                self.advance()
            if depth != 0:
                raise ParseError("Unterminated type annotation block")
        return ReferenceTypeSpec(name=name, generics=generics)

    def parse_function_type_or_grouped_type(self) -> TypeSpec:
        if self.current() != "(":
            raise ParseError("Expected '(' for function type or grouped type")
        self.advance()  # Consume '('
        self.skip_whitespace()

        params: List[TypeSpec] = []
        if self.current() != ")":
            while True:
                self.skip_whitespace()
                save_index = self.index
                try:
                    maybe_name = self.parse_identifier()
                    self.skip_whitespace()
                    if self.current() == ":":
                        self.advance()
                        self.skip_whitespace()
                        param_type = self.parse_type_spec()
                    else:
                        self.index = save_index
                        param_type = self.parse_type_spec()
                except ParseError:
                    self.index = save_index
                    param_type = self.parse_type_spec()

                params.append(param_type)
                self.skip_whitespace()
                if self.current() == ",":
                    self.advance()
                    self.skip_whitespace()
                    continue
                break

        self.skip_whitespace()
        if self.current() != ")":
            raise ParseError("Expected ')' to close function type or grouped type")
        self.advance()  # Consume ')'
        self.skip_whitespace()

        if self.current() == "-" and self.peek(1) == ">":
            self.advance()  # Consume -
            self.advance()  # Consume >
            self.skip_whitespace()
            return_type = self.parse_type_spec()
            return FunctionTypeSpec(parameters=params, return_type=return_type)
        else:
            if len(params) == 1:
                return params[0]
            raise ParseError("Invalid grouped type or malformed function type.")

    def parse_string_literal_type(self) -> LiteralTypeSpec:
        self.advance()
        start = self.index
        while not self.at_end() and self.current() != '"':
            if self.current() == "\\" and self.peek(1) == '"':
                self.advance()
                self.advance()
            else:
                self.advance()
        value = self.source[start : self.index]
        if self.current() != '"':
            raise ParseError("Unterminated string literal type")
        self.advance()
        return LiteralTypeSpec(value=value, base_type=ReferenceTypeSpec(name="String", generics=[]))

    def parse_number_literal_type(self) -> LiteralTypeSpec:
        start = self.index
        if self.current() == "-":
            self.advance()
        while not self.at_end() and self.current().isdigit():
            self.advance()
        if self.current() == ".":
            self.advance()
            while not self.at_end() and self.current().isdigit():
                self.advance()
        number_str = self.source[start : self.index]
        try:
            value = int(number_str) if "." not in number_str else float(number_str)
            return LiteralTypeSpec(value=value, base_type=ReferenceTypeSpec(name="Number", generics=[]))
        except ValueError as e:
            raise ParseError(f"Invalid number literal type: {number_str}") from e

    def parse_boolean_literal_type(self) -> LiteralTypeSpec:
        if self.source[self.index :].startswith("true"):
            self.index += 4
            return LiteralTypeSpec(value=True, base_type=ReferenceTypeSpec(name="Boolean", generics=[]))
        if self.source[self.index :].startswith("false"):
            self.index += 5
            return LiteralTypeSpec(value=False, base_type=ReferenceTypeSpec(name="Boolean", generics=[]))
        raise ParseError("Expected 'true' or 'false' for boolean literal type")

    def parse_object_type(self) -> ObjectTypeSpec:
        if self.current() != "{":
            raise ParseError("Expected '{' for object type")
        self.advance()
        is_closed = False
        if self.current() == "|":
            is_closed = True
            self.advance()
            self.skip_whitespace()

        fields: List[Tuple[str, TypeSpec, bool, bool]] = []
        while not self.at_end():
            self.skip_whitespace()
            if self.current() in {"|", "}"}:
                break

            key = self.parse_identifier()
            is_optional = False
            is_repeatable = False
            self.skip_whitespace()
            if self.current() == "*":
                is_repeatable = True
                self.advance()
                self.skip_whitespace()
            if self.current() == "?":
                is_optional = True
                self.advance()

            self.skip_whitespace()
            if self.current() != ":":
                raise ParseError("Expected ':' in type field definition")
            self.advance()

            field_type = self.parse_type_spec()
            fields.append((key, field_type, is_optional, is_repeatable))

            self.skip_whitespace()
            if self.current() == ",":
                self.advance()
                continue
            if self.current() in {"|", "}"}:
                break
            raise ParseError("Expected ',' or '}' in object type")

        self.skip_whitespace()
        if is_closed:
            if self.current() == "|":
                self.advance()
            if self.current() != "}":
                raise ParseError("Expected '|}' to close object type")
            self.advance()
        else:
            if self.current() != "}":
                raise ParseError("Expected '}' to close object type")
            self.advance()

        return ObjectTypeSpec(fields=fields, is_open=not is_closed)

INFIX_SPECIAL = {
    "map": "_infix_map",
    "reduce": "_infix_reduce",
    "filter": "_infix_filter",
    "flatMap": "_infix_flatMap",
    "distinctBy": "_infix_distinctBy",
    "to": "_infix_to",
    "and": "_binary_and",
    "or": "_binary_or",
}

RESERVED_INFIX_STOP = {
    "else",
    "when",
    "default",
    "match",
    "case",
    "var",
}
