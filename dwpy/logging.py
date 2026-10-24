"""Host-injected, execution-scoped DataWeave loggers."""
from __future__ import annotations

import logging
from collections.abc import Callable, Iterable, Iterator
from contextlib import contextmanager
from contextvars import ContextVar
from dataclasses import dataclass


@dataclass(frozen=True)
class LogEntry:
    level: str
    prefix: str | None
    message: str


Logger = Callable[[LogEntry], None] | logging.Logger
_active: ContextVar[tuple[Logger, ...] | None] = ContextVar("dwpy_loggers", default=None)
_levels = {
    "debug": logging.DEBUG,
    "info": logging.INFO,
    "warn": logging.WARNING,
    "error": logging.ERROR,
}


def normalize_loggers(loggers: Iterable[Logger] | None) -> tuple[Logger, ...] | None:
    if loggers is None:
        return None
    result = tuple(loggers)
    for logger in result:
        if not isinstance(logger, logging.Logger) and not callable(logger):
            raise TypeError(
                "loggers must contain logging.Logger instances or callable LogEntry sinks"
            )
    return result


@contextmanager
def logger_scope(loggers: tuple[Logger, ...] | None) -> Iterator[None]:
    token = _active.set(loggers)
    try:
        yield
    finally:
        _active.reset(token)


def emit_log(entry: LogEntry) -> None:
    configured = _active.get()
    if configured is None:
        # Preserve the legacy Python default, including its WARNING threshold.
        logging.getLogger(__name__).log(_levels[entry.level], entry.message)
        return
    for logger in configured:
        if isinstance(logger, logging.Logger):
            logger.log(_levels[entry.level], entry.message)
        else:
            logger(entry)


def emit_bridge_log(entry: dict) -> None:
    emit_log(LogEntry(**entry))
