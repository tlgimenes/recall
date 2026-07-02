import { CopyButton } from "./CopyButton";

const INSTALL_ROWS: { label: string; cmd: string }[] = [
  {
    label: "1. Install",
    cmd: "curl -fsSL https://github.com/tlgimenes/recall/releases/latest/download/recall-cli-installer.sh | sh",
  },
  { label: "2. Register", cmd: "claude mcp add recall --scope user -- recall mcp" },
];

const SOURCE_ROWS: { label: string; cmd: string }[] = [
  { label: "1. Clone", cmd: "git clone https://github.com/tlgimenes/recall" },
  { label: "2. Build", cmd: "cd recall && cargo build" },
  { label: "3. Register", cmd: "claude mcp add recall -- ./target/debug/recall mcp" },
];

const FUTURE_ROWS: { label: string; cmd: string }[] = [
  { label: "Claude Code", cmd: "/plugin marketplace add tlgimenes/recall" },
  { label: "…then", cmd: "/plugin install recall@recall" },
  { label: "Codex", cmd: "codex plugin marketplace add tlgimenes/recall" },
  { label: "npm / npx", cmd: "npx -y @tlgimenes/recall" },
  { label: "Homebrew", cmd: "brew install tlgimenes/recall/recall" },
];

export function Install() {
  return (
    <section id="install" className="mx-auto max-w-3xl px-6 py-20">
      <h2 className="text-center text-2xl font-bold tracking-tight md:text-3xl">Install</h2>
      <p className="mt-3 text-center text-sm text-[var(--color-muted)]">
        v0.1.0 is out — grab the binary below. The npm, Homebrew, and plugin-store
        packages are still on the way.
      </p>

      <h3 className="mt-10 text-xs font-semibold uppercase tracking-wide text-[var(--color-accent)]">
        Install the binary — works today
      </h3>
      <div className="mt-3 divide-y divide-[var(--color-border)] overflow-hidden rounded-xl border border-[var(--color-accent-dim)] bg-[var(--color-surface)]">
        {INSTALL_ROWS.map((r) => (
          <div key={r.cmd} className="flex items-center gap-3 px-4 py-3">
            <span className="w-24 shrink-0 text-xs text-[var(--color-muted)]">{r.label}</span>
            <code className="flex-1 overflow-x-auto font-mono text-sm">{r.cmd}</code>
            <CopyButton text={r.cmd} />
          </div>
        ))}
      </div>
      <p className="mt-2 text-xs text-[var(--color-muted)]">
        Prebuilt for macOS, Linux, and Windows —{" "}
        <a
          href="https://github.com/tlgimenes/recall/releases/tag/v0.1.0"
          className="underline hover:text-[var(--color-fg)]"
        >
          see the v0.1.0 release
        </a>
        .
      </p>

      <h3 className="mt-10 text-xs font-semibold uppercase tracking-wide text-[var(--color-muted)]">
        Building from source instead
      </h3>
      <div className="mt-3 divide-y divide-[var(--color-border)] overflow-hidden rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]">
        {SOURCE_ROWS.map((r) => (
          <div key={r.cmd} className="flex items-center gap-3 px-4 py-3">
            <span className="w-24 shrink-0 text-xs text-[var(--color-muted)]">{r.label}</span>
            <code className="flex-1 overflow-x-auto font-mono text-sm">{r.cmd}</code>
            <CopyButton text={r.cmd} />
          </div>
        ))}
      </div>
      <p className="mt-2 text-xs text-[var(--color-muted)]">
        Full dogfooding loop, throwaway DBs, and enforcement modes are documented in{" "}
        <a
          href="https://github.com/tlgimenes/recall/blob/main/docs/DEV.md"
          className="underline hover:text-[var(--color-fg)]"
        >
          docs/DEV.md
        </a>
        .
      </p>

      <h3 className="mt-10 text-xs font-semibold uppercase tracking-wide text-[var(--color-muted)]">
        Coming soon — not live yet
      </h3>
      <div className="mt-3 divide-y divide-[var(--color-border)] overflow-hidden rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] opacity-60">
        {FUTURE_ROWS.map((r) => (
          <div key={r.cmd} className="flex items-center gap-3 px-4 py-3">
            <span className="w-28 shrink-0 text-xs text-[var(--color-muted)]">{r.label}</span>
            <code className="flex-1 overflow-x-auto font-mono text-sm">{r.cmd}</code>
            <CopyButton text={r.cmd} />
          </div>
        ))}
      </div>
    </section>
  );
}
