import "./styles/globals.css";
import { BackendStatus } from "./components/BackendStatus";
import { ProcessList } from "./components/ProcessList";
import { SystemOverview } from "./components/SystemOverview";
import { ThemeToggle } from "./components/ThemeToggle";

/**
 * Application shell.
 *
 * The grid is the responsive contract: one column on narrow windows, two at
 * `sm`, and three at `lg`. The process inventory spans the full width because
 * its table needs the room; the other two panels share the remaining columns.
 */
export default function App() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="border-b border-border bg-card">
        <div className="mx-auto flex max-w-7xl flex-wrap items-center justify-between gap-3 px-4 py-3 sm:px-6 lg:px-8">
          <div className="flex flex-wrap items-baseline gap-x-2 gap-y-0.5">
            <span className="text-base font-semibold tracking-tight">System Scanner</span>
            <span className="text-2xs uppercase tracking-widest text-muted-foreground">
              Windows · preview 0.1.0
            </span>
          </div>
          <div className="flex items-center gap-3">
            <span className="text-2xs uppercase tracking-widest text-muted-foreground">
              Read-only · unelevated
            </span>
            <ThemeToggle />
          </div>
        </div>
      </header>

      <main className="mx-auto w-full max-w-7xl px-4 py-6 sm:px-6 lg:px-8">
        <h1 className="sr-only">System overview, backend status, and process inventory</h1>

        <div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
          <SystemOverview className="sm:col-span-2" />
          <BackendStatus />
          <ProcessList className="sm:col-span-2 lg:col-span-3" />
        </div>
      </main>

      <footer className="mx-auto flex max-w-7xl flex-wrap items-center justify-between gap-2 px-4 pb-6 text-2xs uppercase tracking-widest text-muted-foreground sm:px-6 lg:px-8">
        <span>System Scanner / Windows</span>
        <span>No network egress</span>
      </footer>
    </div>
  );
}
