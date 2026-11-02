interface Counts {
  passed: number
  failed: number
  blocked: number
  skipped: number
  untested: number
}

const PARTS: { key: keyof Counts; label: string; color: string }[] = [
  { key: 'passed', label: 'Passed', color: '#16a34a' },
  { key: 'failed', label: 'Failed', color: 'var(--color-accent, #d32f2f)' },
  { key: 'blocked', label: 'Blocked', color: '#f59e0b' },
  { key: 'skipped', label: 'Skipped', color: '#94a3b8' },
  { key: 'untested', label: 'Untested', color: '#e2e8f0' },
]

/** A segmented bar of the status distribution; the counts are in the tooltip and the label (ORCH-026.06). */
export default function StatusBar({ counts, tid }: { counts: Counts; tid?: string }) {
  const total = PARTS.reduce((n, p) => n + counts[p.key], 0)
  const title = PARTS.map((p) => `${p.label} ${counts[p.key]}`).join(' · ')
  return (
    <div data-tid={tid} title={title} aria-label={title} role="img" style={{ display: 'flex', height: 8, gap: 1, background: total === 0 ? '#e2e8f0' : undefined }}>
      {PARTS.filter((p) => counts[p.key] > 0).map((p) => (
        <div key={p.key} data-seg={p.key} style={{ flex: counts[p.key], background: p.color }} />
      ))}
    </div>
  )
}
