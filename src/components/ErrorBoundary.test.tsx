import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ErrorBoundary } from "./ErrorBoundary";

function Boom(): never {
  throw new Error("panel exploded");
}

describe("ErrorBoundary", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders its children when nothing throws", () => {
    render(
      <ErrorBoundary>
        <p>healthy panel</p>
      </ErrorBoundary>,
    );

    expect(screen.getByText("healthy panel")).toBeInTheDocument();
  });

  it("replaces a crashed tree with a readable fatal screen", () => {
    // React logs caught render errors; keep the test output clean.
    vi.spyOn(console, "error").mockImplementation(() => {});

    render(
      <ErrorBoundary>
        <Boom />
      </ErrorBoundary>,
    );

    expect(screen.getByRole("alert")).toHaveTextContent(/hit an unexpected error/i);
    expect(screen.getByText("panel exploded")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /reload window/i })).toBeInTheDocument();
    expect(screen.getByText(/system-scanner\.log/)).toBeInTheDocument();
    expect(screen.queryByText("healthy panel")).not.toBeInTheDocument();
  });
});
