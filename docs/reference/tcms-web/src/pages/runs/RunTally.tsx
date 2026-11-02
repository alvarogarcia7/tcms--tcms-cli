import { STATUS_META, STATUS_ORDER, ViewStatus } from './model'

interface Props {
  counts: Record<ViewStatus, number>
  total: number
  filter: ReadonlySet<ViewStatus>
  onToggle: (status: ViewStatus) => void
  onReset: () => void
}

/** The segmented progress bar and the status chips; a chip filters the list, board and focus queue. */
export default function RunTally({ counts, total, filter, onToggle, onReset }: Props) {
  return (
    <>
      <div data-tid="runs-progress" className="run-progress">
        {STATUS_ORDER.map((s) => {
          const n = counts[s]
          if (n === 0) return null
          return <div key={s} style={{ flex: n, background: STATUS_META[s].bar }} />
        })}
      </div>

      <div data-tid="runs-counts" className="run-filters">
        <span className="run-label run-filter-label">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
            <path d="M22 3H2l8 9.46V19l4 2v-8.54L22 3z"></path>
          </svg>
          Filter
        </span>
        {STATUS_ORDER.map((s) => {
          const active = filter.has(s)
          return (
            <button
              key={s}
              type="button"
              data-tid={`runs-count-${s}`}
              aria-pressed={active}
              title={`Show only ${STATUS_META[s].label.toLowerCase()} tests`}
              onClick={() => onToggle(s)}
              className="run-chip"
            >
              <span className="run-swatch" style={{ background: STATUS_META[s].bg, borderColor: STATUS_META[s].border }} />
              {STATUS_META[s].label} <strong>{counts[s]}</strong>
            </button>
          )
        })}
        <span data-tid="runs-total" className="run-total">
          {total} {total === 1 ? 'case' : 'cases'}
        </span>
        {filter.size > 0 && (
          <button type="button" data-tid="runs-filter-reset" className="btn btn-secondary run-small-btn" onClick={onReset}>
            Reset
          </button>
        )}
      </div>
    </>
  )
}
