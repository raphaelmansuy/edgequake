"""Pagination utilities for the EdgeQuake SDK.

WHY: Auto-paginating iterators provide a clean API for callers — they just
iterate over results and the SDK handles page fetching transparently.
This eliminates boilerplate pagination loops in user code.
"""

from __future__ import annotations

from collections.abc import AsyncIterator, Awaitable, Callable, Iterator
from typing import (
    Generic,
    TypeVar,
)

from pydantic import BaseModel

T = TypeVar("T", bound=BaseModel)


class PaginatedResponse(BaseModel, Generic[T]):
    """Standard paginated response from the EdgeQuake API.

    Matches the server's JSON response format:
    { "items": [...], "total": N, "page": M, "page_size": K }
    """

    items: list[T]
    total: int = 0
    page: int = 1
    page_size: int = 50

    @property
    def has_next(self) -> bool:
        """True if there are more pages available."""
        return self.page * self.page_size < self.total


class SyncPaginator(Generic[T]):
    """Auto-paginating synchronous iterator.

    Transparently fetches pages as the caller iterates. Supports convenience
    methods like to_list() and first().

    Usage:
        for item in paginator:
            print(item)

        all_items = paginator.to_list()
        first_item = paginator.first()
    """

    def __init__(
        self,
        fetch_page: Callable[[int, int], PaginatedResponse[T]],
        *,
        page_size: int = 50,
    ) -> None:
        self._fetch_page = fetch_page
        self._page_size = page_size
        self._current_page = 0
        self._items: list[T] = []
        self._index = 0
        self._exhausted = False

    def __iter__(self) -> Iterator[T]:
        return self

    def __next__(self) -> T:
        if self._index >= len(self._items):
            if self._exhausted:
                raise StopIteration
            self._load_next_page()
        if self._index >= len(self._items):
            raise StopIteration
        item = self._items[self._index]
        self._index += 1
        return item

    def _load_next_page(self) -> None:
        """Fetch the next page of results."""
        self._current_page += 1
        response = self._fetch_page(self._current_page, self._page_size)
        self._items.extend(response.items)
        if not response.items or len(response.items) < self._page_size:
            self._exhausted = True

    def to_list(self) -> list[T]:
        """Collect all remaining items into a list."""
        return list(self)

    def first(self) -> T | None:
        """Return the first item or None."""
        try:
            return next(iter(self))
        except StopIteration:
            return None

    def with_page_size(self, size: int) -> SyncPaginator[T]:
        """Return a new paginator with a different page size."""
        return SyncPaginator(self._fetch_page, page_size=size)


class AsyncPaginator(Generic[T]):
    """Auto-paginating asynchronous iterator.

    Same behavior as SyncPaginator but for async/await usage.

    Usage:
        async for item in paginator:
            print(item)

        all_items = await paginator.to_list()
    """

    def __init__(
        self,
        fetch_page: Callable[[int, int], Awaitable[PaginatedResponse[T]]],
        *,
        page_size: int = 50,
    ) -> None:
        self._fetch_page = fetch_page
        self._page_size = page_size
        self._current_page = 0
        self._items: list[T] = []
        self._index = 0
        self._exhausted = False

    def __aiter__(self) -> AsyncIterator[T]:
        return self

    async def __anext__(self) -> T:
        if self._index >= len(self._items):
            if self._exhausted:
                raise StopAsyncIteration
            await self._load_next_page()
        if self._index >= len(self._items):
            raise StopAsyncIteration
        item = self._items[self._index]
        self._index += 1
        return item

    async def _load_next_page(self) -> None:
        """Fetch the next page of results."""
        self._current_page += 1
        response = await self._fetch_page(self._current_page, self._page_size)
        self._items.extend(response.items)
        if not response.items or len(response.items) < self._page_size:
            self._exhausted = True

    async def to_list(self) -> list[T]:
        """Collect all remaining items into a list."""
        return [item async for item in self]
