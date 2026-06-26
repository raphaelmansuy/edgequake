/**
 * SPEC-021 stabilization — ApiErrorBoundary isolates render failures.
 *
 * Tested at the class-method level (no DOM) because the project's vitest
 * config runs in the `node` environment without @testing-library/react.
 */
import { describe, expect, it, vi } from "vitest";
import { ApiErrorBoundary } from "../api-error-boundary";

describe("ApiErrorBoundary", () => {
  it("getDerivedStateFromError stores the error", () => {
    const state = ApiErrorBoundary.getDerivedStateFromError(new Error("boom"));
    expect(state.error).toBeInstanceOf(Error);
    expect((state.error as Error).message).toBe("boom");
  });

  it("componentDidCatch calls onError and logs via console.warn (not error)", () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    const onError = vi.fn();

    const boundary = new ApiErrorBoundary({ onError });
    boundary.setState = vi.fn();
    boundary.componentDidCatch(new Error("boom"), {
      componentStack: "<Boom>",
    });

    expect(onError).toHaveBeenCalledOnce();
    expect(warnSpy).toHaveBeenCalled();
    expect(errorSpy).not.toHaveBeenCalled();
    warnSpy.mockRestore();
    errorSpy.mockRestore();
  });

  it("retry clears the error state", () => {
    const boundary = new ApiErrorBoundary({});
    const setStateSpy = vi.spyOn(boundary, "setState");
    boundary.retry();
    expect(setStateSpy).toHaveBeenCalledWith({ error: null });
  });

  it("renders children when there is no error", () => {
    const boundary = new ApiErrorBoundary({ children: "child" });
    expect(boundary.render()).toBe("child");
  });

  it("renders default fallback when an error is set", () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const boundary = new ApiErrorBoundary({});
    boundary.state = { error: new Error("boom") };
    const result = boundary.render() as { props: { error: Error } };
    // Default fallback is a React element; verify it rendered (not null/undefined)
    expect(result).toBeTruthy();
    warnSpy.mockRestore();
  });
});
