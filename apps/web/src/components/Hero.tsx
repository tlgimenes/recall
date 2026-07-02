import { CopyButton } from "./CopyButton";
import { Terminal } from "./Terminal";

const GH = "https://github.com/tlgimenes/recall";
const INSTALL =
  "curl -fsSL https://github.com/tlgimenes/recall/releases/latest/download/recall-cli-installer.sh | sh";

export function Hero() {
  return (
    <header className="mx-auto max-w-6xl px-6 pt-20 pb-12 md:pt-28">
      <div className="mx-auto max-w-3xl text-center">
        <div className="mb-4 inline-flex items-center gap-2 rounded-full border border-[var(--color-border)] px-3 py-1 font-mono text-xs text-[var(--color-muted)]">
          <span className="text-[var(--color-accent)]">●</span> local-first · MCP · Rust
        </div>
        <h1 className="text-balance text-4xl font-bold tracking-tight md:text-6xl">
          Teach your AI once.
          <br />
          It writes code <span className="text-[var(--color-accent)]">like you</span> everywhere.
        </h1>
        <p className="mx-auto mt-6 max-w-2xl text-balance text-lg text-[var(--color-muted)]">
          Recall is a personal convention brain for coding agents. Correct your AI
          once — it remembers and applies it in every repo, every branch, and every
          agent. Claude Code, Codex, and beyond.
        </p>
        <div className="mt-8 flex flex-col items-center justify-center gap-3 sm:flex-row">
          <div className="flex items-center gap-3 overflow-x-auto rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] px-4 py-2.5 font-mono text-sm">
            <span className="text-[var(--color-muted)]">$</span>
            <span>{INSTALL}</span>
            <CopyButton text={INSTALL} />
          </div>
          <a
            href={GH}
            className="rounded-lg bg-[var(--color-accent)] px-4 py-2.5 text-sm font-semibold text-black transition hover:opacity-90"
          >
            Star on GitHub →
          </a>
        </div>
        <p className="mt-3 text-xs text-[var(--color-muted)]">
          v0.1.0 — prebuilt for macOS, Linux, and Windows. See{" "}
          <a href="#install" className="underline hover:text-[var(--color-fg)]">
            Install
          </a>{" "}
          to register it with your agent, or build from source.
        </p>
      </div>
      <div className="mt-14">
        <Terminal />
      </div>
    </header>
  );
}
