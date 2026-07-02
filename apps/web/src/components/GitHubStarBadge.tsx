import { StarCount } from "./StarCount";

export function GitHubStarBadge() {
  return (
    <a
      href="https://github.com/tlgimenes/recall"
      className="fixed right-4 top-4 z-50 flex items-center gap-1.5 rounded-full border border-[var(--color-border)] bg-[var(--color-surface)]/90 px-3 py-1.5 font-mono text-xs text-[var(--color-muted)] backdrop-blur transition hover:border-[var(--color-accent-dim)] hover:text-[var(--color-fg)]"
    >
      GitHub
      <StarCount />
    </a>
  );
}
