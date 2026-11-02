import { CaseStatus, RecordedRunCase, RunCase, RunCaseSummary } from '../../types/runs'

/** The five statuses the screen shows, in the order of the progress bar, tally chips and board. */
export type ViewStatus = 'pass' | 'fail' | 'blocked' | 'skip' | 'pending'
export type ViewMode = 'board' | 'focus' | 'split'

/** Labels and colours of the design's statuses (S in design/TCMS Prototype.dc.html): passed is ink,
 *  failed the accent, blocked a light accent, skipped grey, untested an empty box. `bar` is the
 *  colour of the status in the progress bar (untested shows as grey there). */
export const STATUS_META: Record<ViewStatus, { label: string; bg: string; border: string; bar: string }> = {
  pass: { label: 'Passed', bg: 'var(--color-neutral-900)', border: 'var(--color-neutral-900)', bar: 'var(--color-neutral-900)' },
  fail: { label: 'Failed', bg: 'var(--color-accent)', border: 'var(--color-accent)', bar: 'var(--color-accent)' },
  blocked: { label: 'Blocked', bg: 'var(--color-accent-300)', border: 'var(--color-accent-400)', bar: 'var(--color-accent-300)' },
  skip: { label: 'Skipped', bg: 'var(--color-neutral-300)', border: 'var(--color-neutral-400)', bar: 'var(--color-neutral-300)' },
  pending: { label: 'Untested', bg: 'transparent', border: 'var(--color-neutral-500)', bar: 'var(--color-neutral-300)' },
}

export const STATUS_ORDER: ViewStatus[] = ['pass', 'fail', 'blocked', 'skip', 'pending']

export interface ViewStep {
  n: number
  action: string
  expected: string
  status: ViewStatus
}

/** One case of the run on screen, whether it comes from a plan run or from the loaded V&V run. */
export interface ViewCase {
  /** Unique within the run; the plan run's case row id or the V&V case id. */
  key: string
  id: string
  title: string
  type: string
  priority: string | null
  status: ViewStatus
  /** Steps when already known (V&V); a plan run's steps come with the case detail. */
  steps: ViewStep[] | null
  /** CI-reported cases cannot be set by hand. */
  automated: boolean
  /** The plan run's case row id, absent for V&V cases. */
  rowId: number | null
}

const FROM_API: Record<CaseStatus, ViewStatus> = {
  PASSED: 'pass',
  FAILED: 'fail',
  BLOCKED: 'blocked',
  SKIPPED: 'skip',
  UNTESTED: 'pending',
}

const TO_API: Record<ViewStatus, CaseStatus> = {
  pass: 'PASSED',
  fail: 'FAILED',
  blocked: 'BLOCKED',
  skip: 'SKIPPED',
  pending: 'UNTESTED',
}

export const fromApiStatus = (status: CaseStatus): ViewStatus => FROM_API[status] ?? 'pending'
export const toApiStatus = (status: ViewStatus): CaseStatus => TO_API[status]

export function viewCaseOf(c: RunCaseSummary): ViewCase {
  return {
    key: String(c.id),
    id: c.caseId ?? c.file,
    title: c.title ?? c.file,
    type: c.type ? c.type.charAt(0).toUpperCase() + c.type.slice(1) : 'Manual',
    priority: c.priority,
    status: fromApiStatus(c.status),
    steps: null,
    automated: c.automated,
    rowId: c.id,
  }
}

export function stepsOf(detail: RunCase): ViewStep[] {
  return detail.steps.map((s) => ({ n: s.n, action: s.action, expected: s.expected, status: fromApiStatus(s.status) }))
}

/** Counts per status. */
export function tallyOf(cases: ViewCase[]): Record<ViewStatus, number> {
  const base: Record<ViewStatus, number> = { pass: 0, fail: 0, blocked: 0, skip: 0, pending: 0 }
  for (const c of cases) base[c.status] += 1
  return base
}

/** A case of a recorded V&V run (read-only, D-433): no row id, so nothing can be recorded on it. */
export function viewCaseOfRecorded(c: RecordedRunCase): ViewCase {
  return {
    key: `vv-${c.testCaseId}`,
    id: c.testCaseId,
    title: c.title || c.testCaseId,
    type: 'Automated',
    priority: null,
    status: fromApiStatus(c.status),
    steps: c.steps.map((s) => ({
      n: s.n,
      action: s.description ?? `Step ${s.step ?? s.n}`,
      expected: s.reason ?? '',
      status: fromApiStatus(s.status),
    })),
    automated: false,
    rowId: null,
  }
}
