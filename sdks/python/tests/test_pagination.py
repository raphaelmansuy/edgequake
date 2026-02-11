"""Tests for edgequake._pagination module."""

from __future__ import annotations

import pytest
from pydantic import BaseModel

from edgequake._pagination import AsyncPaginator, PaginatedResponse, SyncPaginator


class Item(BaseModel):
    """Simple test item."""

    id: int
    name: str


class TestPaginatedResponse:
    """Test PaginatedResponse model."""

    def test_basic_response(self) -> None:
        resp = PaginatedResponse[Item](
            items=[Item(id=1, name="a"), Item(id=2, name="b")],
            total=5,
            page=1,
            page_size=2,
        )
        assert len(resp.items) == 2
        assert resp.total == 5
        assert resp.page == 1
        assert resp.page_size == 2

    def test_has_next_true(self) -> None:
        resp = PaginatedResponse[Item](
            items=[Item(id=1, name="a")],
            total=10,
            page=1,
            page_size=5,
        )
        assert resp.has_next is True

    def test_has_next_false_last_page(self) -> None:
        resp = PaginatedResponse[Item](
            items=[Item(id=1, name="a")],
            total=5,
            page=1,
            page_size=5,
        )
        assert resp.has_next is False

    def test_has_next_false_exact(self) -> None:
        resp = PaginatedResponse[Item](
            items=[Item(id=1, name="a"), Item(id=2, name="b")],
            total=4,
            page=2,
            page_size=2,
        )
        assert resp.has_next is False

    def test_empty_response(self) -> None:
        resp = PaginatedResponse[Item](items=[], total=0)
        assert resp.has_next is False
        assert len(resp.items) == 0

    def test_defaults(self) -> None:
        resp = PaginatedResponse[Item](items=[])
        assert resp.total == 0
        assert resp.page == 1
        assert resp.page_size == 50


class TestSyncPaginator:
    """Test synchronous auto-paginating iterator."""

    def _make_fetch(self, pages: list[list[Item]], total: int):
        """Create a fetch_page function that returns pre-defined pages."""

        def fetch(page: int, page_size: int) -> PaginatedResponse[Item]:
            idx = page - 1
            if idx < len(pages):
                return PaginatedResponse[Item](
                    items=pages[idx],
                    total=total,
                    page=page,
                    page_size=page_size,
                )
            return PaginatedResponse[Item](
                items=[], total=total, page=page, page_size=page_size
            )

        return fetch

    def test_iterate_single_page(self) -> None:
        items = [Item(id=1, name="a"), Item(id=2, name="b")]
        paginator = SyncPaginator(self._make_fetch([items], 2), page_size=10)
        result = list(paginator)
        assert len(result) == 2
        assert result[0].id == 1
        assert result[1].id == 2

    def test_iterate_multiple_pages(self) -> None:
        page1 = [Item(id=1, name="a"), Item(id=2, name="b")]
        page2 = [Item(id=3, name="c"), Item(id=4, name="d")]
        page3 = [Item(id=5, name="e")]
        paginator = SyncPaginator(
            self._make_fetch([page1, page2, page3], 5), page_size=2
        )
        result = list(paginator)
        assert len(result) == 5
        assert [r.id for r in result] == [1, 2, 3, 4, 5]

    def test_empty_first_page(self) -> None:
        paginator = SyncPaginator(self._make_fetch([[]], 0), page_size=10)
        result = list(paginator)
        assert result == []

    def test_to_list(self) -> None:
        items = [Item(id=1, name="a")]
        paginator = SyncPaginator(self._make_fetch([items], 1), page_size=10)
        result = paginator.to_list()
        assert len(result) == 1
        assert result[0].id == 1

    def test_first_returns_item(self) -> None:
        items = [Item(id=42, name="first")]
        paginator = SyncPaginator(self._make_fetch([items], 1), page_size=10)
        item = paginator.first()
        assert item is not None
        assert item.id == 42

    def test_first_returns_none_when_empty(self) -> None:
        paginator = SyncPaginator(self._make_fetch([[]], 0), page_size=10)
        assert paginator.first() is None

    def test_with_page_size(self) -> None:
        items = [Item(id=1, name="a")]
        fetch = self._make_fetch([items], 1)
        paginator = SyncPaginator(fetch, page_size=10)
        new_paginator = paginator.with_page_size(5)
        assert new_paginator is not paginator
        assert new_paginator._page_size == 5


@pytest.mark.asyncio
class TestAsyncPaginator:
    """Test asynchronous auto-paginating iterator."""

    def _make_async_fetch(self, pages: list[list[Item]], total: int):
        """Create async fetch_page function."""

        async def fetch(page: int, page_size: int) -> PaginatedResponse[Item]:
            idx = page - 1
            if idx < len(pages):
                return PaginatedResponse[Item](
                    items=pages[idx],
                    total=total,
                    page=page,
                    page_size=page_size,
                )
            return PaginatedResponse[Item](
                items=[], total=total, page=page, page_size=page_size
            )

        return fetch

    async def test_iterate_single_page(self) -> None:
        items = [Item(id=1, name="a")]
        paginator = AsyncPaginator(self._make_async_fetch([items], 1), page_size=10)
        result = await paginator.to_list()
        assert len(result) == 1

    async def test_iterate_multiple_pages(self) -> None:
        page1 = [Item(id=1, name="a"), Item(id=2, name="b")]
        page2 = [Item(id=3, name="c")]
        paginator = AsyncPaginator(
            self._make_async_fetch([page1, page2], 3), page_size=2
        )
        result = await paginator.to_list()
        assert len(result) == 3
        assert [r.id for r in result] == [1, 2, 3]

    async def test_empty_result(self) -> None:
        paginator = AsyncPaginator(self._make_async_fetch([[]], 0), page_size=10)
        result = await paginator.to_list()
        assert result == []
