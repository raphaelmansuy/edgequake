"""Python version compatibility helpers for the EdgeQuake SDK.

WHY: Provide consistent behavior across Python 3.10-3.13 by centralizing
version-specific imports and polyfills in one place.
"""

from __future__ import annotations

import sys

# WHY: Python 3.11 introduced ExceptionGroup; provide fallback for 3.10
if sys.version_info >= (3, 11):
    from typing import Self
else:
    from typing import TypeVar

    Self = TypeVar("Self")  # type: ignore[misc,assignment]
