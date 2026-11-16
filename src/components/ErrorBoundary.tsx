import { Component, type ErrorInfo, type ReactNode } from "react";
import { Button } from "./ui/button";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

/**
 * Catches a render failure and replaces the dashboard with a readable screen.
 *
 * React unmounts the whole tree when a render throws, so without a boundary the
 * window would go blank. The fallback is deliberately self-contained: no IPC, no
 * clipboard, no retry loop. It reports that the failure is a bug, points at the
 * log file, and offers a reload.
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    // There is no telemetry and no IPC on this path. Keep the detail in the
    // developer console so a local run is diagnosable.
    console.error("System Scanner render failure", error, info.componentStack);
  }

  render() {
    const { error } = this.state;
    if (error === null) {
      return this.props.children;
    }

    return (
      <main className="flex min-h-screen items-center justify-center bg-background px-6 py-12 text-foreground">
        <div role="alert" className="w-full max-w-xl space-y-4">
          <h1 className="text-xl font-semibold tracking-tight">
            System Scanner hit an unexpected error
          </h1>
          <p className="text-sm text-muted-foreground">
            The window stopped rendering. This is a bug in the app, not a problem with your machine,
            and System Scanner only reads system information, so your files were not changed.
          </p>
          <pre className="max-h-40 overflow-auto whitespace-pre-wrap break-words rounded-md border border-border bg-card p-3 font-mono text-xs text-foreground">
            {error.message || "No error detail was provided."}
          </pre>
          <div className="flex flex-wrap items-center gap-3">
            <Button size="sm" onClick={() => window.location.reload()}>
              Reload window
            </Button>
            <span className="text-xs text-muted-foreground">
              If this repeats, the log file <code>system-scanner.log</code> in the app log folder
              records what happened before the window failed.
            </span>
          </div>
        </div>
      </main>
    );
  }
}
