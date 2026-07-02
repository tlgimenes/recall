import { useEffect, useState } from "react";

const REPO_API = "https://api.github.com/repos/tlgimenes/recall";

function formatStars(n: number): string {
  return n >= 1000 ? `${(n / 1000).toFixed(1).replace(/\.0$/, "")}k` : String(n);
}

export function StarCount() {
  const [stars, setStars] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    fetch(REPO_API)
      .then((res) => (res.ok ? res.json() : Promise.reject(res.status)))
      .then((data: { stargazers_count?: number }) => {
        if (!cancelled && typeof data.stargazers_count === "number") {
          setStars(data.stargazers_count);
        }
      })
      .catch(() => {
        // Silently omit the count on any failure (rate limit, offline, etc.)
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (stars === null) return null;

  return <span aria-label={`${stars} GitHub stars`}>★ {formatStars(stars)}</span>;
}
