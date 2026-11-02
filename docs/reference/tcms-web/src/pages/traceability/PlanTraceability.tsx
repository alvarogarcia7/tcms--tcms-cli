import { useEffect, useMemo, useState } from 'react'
import { jiraLine } from '../../utils/jiraLine'
import { Run, CaseStatus } from '../../types/runs'
import { User, Workspace } from '../../types'
import { Coverage, TraceRequirement, Traceability } from '../../types/traceability'
import Header from '../../components/Header'
import { traceApi } from '../../utils/traceApi'
import { SelectedCase } from '../../utils/selectedCase'
import TraceabilityDocumentButton from '../../components/TraceabilityDocumentButton'
import RunTestsButton from '../../components/RunTestsButton'
import { canAdminister, canManage, currentRole } from '../../utils/roles'
import { useProduct } from '../../utils/product'
import { useJiraProject } from '../../utils/useJiraProject'
import '../../styles/traceability.css'

interface Props {
  user: User
  workspaces?: Workspace[]
  currentWorkspaceId?: number | null
  onSelectWorkspace?: (workspace: Workspace) => void
  onNavigate: (screen: string) => void
  onSignOut: () => void
  onOpenCase?: (selected: SelectedCase | null) => void
  /** Empty before the first run of a plan: the requirements show all "Not run" (D-130). */
  runs: Run[]
  /** The product (test repository) to show when there is no run. */
  repositoryId?: number | null
}

const COVERAGE: Record<Coverage, { label: string; className: string; rank: number }> = {
  FAILING: { label: 'Failing', className: 'bg-red-100 text-red-800', rank: 0 },
  NOT_RUN: { label: 'Not run', className: 'bg-yellow-100 text-yellow-800', rank: 1 },
  NO_CASES: { label: 'No cases', className: 'bg-neutral-100 text-neutral-700', rank: 2 },
  COVERED: { label: 'Covered', className: 'bg-green-100 text-green-800', rank: 3 },
}

const RESULT: Record<CaseStatus, { bg: string; bd: string; label: string }> = {
  PASSED: { bg: '#dcfce7', bd: '#16a34a', label: 'Passed' },
  FAILED: { bg: '#fee2e2', bd: '#dc2626', label: 'Failed' },
  BLOCKED: { bg: '#fef3c7', bd: '#d97706', label: 'Blocked' },
  SKIPPED: { bg: '#e2e8f0', bd: '#94a3b8', label: 'Skipped' },
  UNTESTED: { bg: '#fff', bd: '#cbd5e1', label: 'Untested' },
}

type SortKey = 'requirement' | 'coverage' | 'defects'

/** Requirement traceability over plan runs (ORCH-018): coverage of each requirement by one run. */
export default function PlanTraceability({ user, workspaces = [], currentWorkspaceId, onSelectWorkspace, onNavigate, onSignOut, onOpenCase, runs, repositoryId }: Props) {
  const [runId, setRunId] = useState<number | null>(runs.find((r) => r.status === 'OPEN')?.id ?? runs[0]?.id ?? null)
  const run: Run | undefined = runs.find((r) => r.id === runId) ?? runs[0]
  const [data, setData] = useState<Traceability | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [coverage, setCoverage] = useState<'ALL' | Coverage>('ALL')
  const [defectFilter, setDefectFilter] = useState<'ALL' | 'WITH' | 'WITHOUT'>('ALL')
  const [search, setSearch] = useState('')
  const [sort, setSort] = useState<{ key: SortKey; dir: 1 | -1 }>({ key: 'requirement', dir: 1 })

  useEffect(() => {
    let alive = true
    setData(null)
    setError(null)
    traceApi
      .get(run ? { runId: run.id } : { repositoryId: repositoryId ?? undefined })
      .then((d) => alive && setData(d))
      .catch((e) => alive && setError(e instanceof Error ? e.message : 'Failed to load the traceability'))
    return () => {
      alive = false
    }
  }, [run?.id, repositoryId])

  const rows = useMemo(() => {
    const q = search.trim().toLowerCase()
    const list = (data?.requirements ?? []).filter((r) => {
      if (coverage !== 'ALL' && r.coverage !== coverage) return false
      if (defectFilter === 'WITH' && r.defects.length === 0) return false
      if (defectFilter === 'WITHOUT' && r.defects.length > 0) return false
      if (q && !`${r.key} ${r.title ?? ''}`.toLowerCase().includes(q)) return false
      return true
    })
    const by = (r: TraceRequirement) => (sort.key === 'requirement' ? r.key : sort.key === 'coverage' ? COVERAGE[r.coverage].rank : r.defects.filter((d) => d.open).length)
    return [...list].sort((a, b) => {
      const x = by(a)
      const y = by(b)
      const c = typeof x === 'number' && typeof y === 'number' ? x - y : String(x).localeCompare(String(y), undefined, { numeric: true })
      return c * sort.dir || a.key.localeCompare(b.key, undefined, { numeric: true })
    })
  }, [data, coverage, defectFilter, search, sort])

  const toggleSort = (key: SortKey) => setSort((s) => (s.key === key ? { key, dir: s.dir === 1 ? -1 : 1 } : { key, dir: 1 }))
  const arrow = (key: SortKey) => (sort.key === key ? (sort.dir === 1 ? ' ↑' : ' ↓') : '')

  const runLabel = run ? `run-${run.number}` : 'no-run'
  const exportCsv = async () => {
    try {
      const blob = await traceApi.exportCsv(run ? { runId: run.id } : { repositoryId: repositoryId ?? undefined })
      const url = URL.createObjectURL(blob)
      const link = document.createElement('a')
      link.href = url
      link.download = `traceability_${runLabel}_${new Date().toISOString().slice(0, 10)}.csv`
      document.body.appendChild(link)
      link.click()
      link.remove()
      URL.revokeObjectURL(url)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Could not export the matrix')
    }
  }

  const exportPdf = () => {
    // The browser's print dialog saves a PDF; the title becomes the default file name (D-100, D-552).
    const previous = document.title
    document.title = `traceability_${runLabel}_${new Date().toISOString().slice(0, 10)}`
    const restore = () => {
      document.title = previous
      window.removeEventListener('afterprint', restore)
    }
    window.addEventListener('afterprint', restore)
    window.print()
  }

  const { productId } = useProduct()
  const jiraProject = useJiraProject(currentWorkspaceId)
  const documentRepositoryId = data?.repository?.id ?? productId ?? null
  const summary = data?.summary
  const th = 'px-6 py-3 text-left text-sm font-semibold text-neutral-900'

  return (
    <div className="min-h-screen bg-neutral-50" data-tid="traceability-screen" data-screen-label="Traceability">
      <Header user={user} workspaces={workspaces} currentWorkspaceId={currentWorkspaceId} onSelectWorkspace={onSelectWorkspace} currentPage="traceability" onNavigate={onNavigate} onSignOut={onSignOut} />

      <main className="max-w-7xl mx-auto px-6 py-8">
        <div className="space-y-6">
          <div data-tid="traceability-header" className="flex items-end justify-between gap-4 flex-wrap pb-4 border-b-2 border-neutral-200">
            <div>
              <div data-tid="traceability-context" className="text-xs uppercase tracking-wider text-neutral-500">
                {data ? `${data.repository.name} · ` : ''}
                {run ? `Run #${run.number} · ${run.planName}` : 'No run yet'}
                {data && !data.jiraConfigured ? ' · Jira not connected' : jiraProject ? ` · Jira project ${jiraProject}` : ''}
              </div>
              <h1 className="mt-1 text-3xl font-bold text-neutral-900">Traceability</h1>
            </div>
            <div className="flex items-end gap-4 flex-wrap">
              {run && (
              <label className="text-xs uppercase tracking-wider text-neutral-500">
                Run{' '}
                <select data-tid="traceability-run-select" value={run.id} onChange={(e) => setRunId(Number(e.target.value))}>
                  {runs.map((r) => (
                    <option key={r.id} value={r.id}>
                      #{r.number} · {r.planName}
                      {r.status === 'OPEN' ? ' (open)' : ''}
                    </option>
                  ))}
                </select>
              </label>
              )}
              <span className="trace-noprint flex items-end gap-2">
                <button type="button" data-tid="traceability-export-pdf" className="btn btn-primary" onClick={exportPdf} disabled={!data} title="Prints the matrix: choose Save as PDF">
                  Export PDF
                </button>
                <button type="button" data-tid="traceability-export" className="btn btn-primary" onClick={exportCsv} disabled={!data}>
                  Export CSV
                </button>
                <TraceabilityDocumentButton repositoryId={documentRepositoryId} canGenerate={canManage(currentRole(workspaces, currentWorkspaceId))} canSetUp={canAdminister(currentRole(workspaces, currentWorkspaceId))} onNavigate={onNavigate} />
                <RunTestsButton repositoryId={documentRepositoryId} role={currentRole(workspaces, currentWorkspaceId)} place="traceability" primary onNavigate={onNavigate} />
              </span>
            </div>
          </div>

          {error && (
            <div data-tid="traceability-error" role="alert" className="text-sm" style={{ color: '#8a1f17' }}>
              {error}
            </div>
          )}
          {!data && !error && <p data-tid="traceability-loading">Loading traceability...</p>}

          {summary && (
            <div data-tid="coverage-summary" className="flex gap-6 text-sm flex-wrap">
              <div data-tid="trace-sum-covered">
                <strong className="text-2xl font-semibold text-neutral-900">
                  {summary.covered}/{summary.total}
                </strong>{' '}
                requirements covered
              </div>
              <div data-tid="trace-sum-notrun">
                <strong className="text-2xl font-semibold">{summary.notRun}</strong> not run
              </div>
              <div data-tid="trace-sum-failing">
                <strong className="text-2xl font-semibold" style={{ color: 'var(--color-accent)' }}>
                  {summary.failing}
                </strong>{' '}
                failing
              </div>
              <div data-tid="trace-sum-nocases">
                <strong className="text-2xl font-semibold">{summary.noCases}</strong> no cases
              </div>
              <div data-tid="trace-sum-withdefect">
                <strong className="text-2xl font-semibold">{summary.withDefect}</strong> with a defect
              </div>
              <div data-tid="trace-sum-opendefects">
                <strong className="text-2xl font-semibold" style={{ color: 'var(--color-accent)' }}>
                  {summary.openDefects}
                </strong>{' '}
                open defects
              </div>
            </div>
          )}

          {data && (
            <div data-tid="traceability-filters" className="flex gap-3 flex-wrap items-center">
              <input data-tid="traceability-search" className="input" placeholder="Search requirement key or title" value={search} onChange={(e) => setSearch(e.target.value)} />
              <label className="text-xs uppercase tracking-wider text-neutral-500">
                Coverage{' '}
                <select data-tid="traceability-coverage-filter" value={coverage} onChange={(e) => setCoverage(e.target.value as 'ALL' | Coverage)}>
                  <option value="ALL">All</option>
                  <option value="COVERED">Covered</option>
                  <option value="NOT_RUN">Not run</option>
                  <option value="FAILING">Failing</option>
                  <option value="NO_CASES">No cases</option>
                </select>
              </label>
              <label className="text-xs uppercase tracking-wider text-neutral-500">
                Defects{' '}
                <select data-tid="traceability-defect-filter" value={defectFilter} onChange={(e) => setDefectFilter(e.target.value as 'ALL' | 'WITH' | 'WITHOUT')}>
                  <option value="ALL">All</option>
                  <option value="WITH">With a defect</option>
                  <option value="WITHOUT">Without a defect</option>
                </select>
              </label>
              <span data-tid="traceability-count" className="text-sm text-neutral-600">
                {rows.length} of {data.requirements.length} requirements
              </span>
              {data.requirements.length === 0 && (
                <span data-tid="traceability-filters-note" className="text-sm text-neutral-600">
                  The filters have nothing to filter yet: no requirement is listed in your test cases.
                </span>
              )}
            </div>
          )}

          <div data-tid="traceability-legend" className="flex items-center gap-4 text-xs text-neutral-600 flex-wrap">
            <span className="font-medium text-neutral-700">{run ? `Colour key for the results in run #${run.number}:` : 'No run yet: every linked case is not run.'}</span>
            {(Object.keys(RESULT) as CaseStatus[]).map((s) => (
              <span key={s} className="flex items-center gap-1.5">
                <span className="inline-block" style={{ width: 14, height: 14, background: RESULT[s].bg, border: `1px solid ${RESULT[s].bd}` }} />
                {RESULT[s].label}
              </span>
            ))}
          </div>

          {data && data.requirements.length === 0 && (
            <div data-tid="traceability-no-data" className="bg-white rounded-lg border border-neutral-200 px-6 py-12 text-center text-sm text-neutral-500">
              No requirements to show yet for {data.repository.name}. A requirement shows here when a test case lists it. Open a test case in your repository, add a <code>requirements</code> list to it, like <code>requirements: [REQ-1]</code>, then refresh.
            </div>
          )}

          {data && data.requirements.length > 0 && (
            <div data-tid="requirements-matrix" className="bg-white rounded-lg border border-neutral-200 overflow-hidden">
              <table className="w-full">
                <thead className="bg-neutral-50 border-b border-neutral-200">
                  <tr>
                    <th className={th}>
                      <button type="button" data-tid="traceability-sort-requirement" onClick={() => toggleSort('requirement')}>
                        Requirement{arrow('requirement')}
                      </button>
                    </th>
                    <th className={th}>Test cases</th>
                    <th className={th}>Latest result</th>
                    <th className={th}>
                      <button type="button" data-tid="traceability-sort-defects" onClick={() => toggleSort('defects')}>
                        Defects{arrow('defects')}
                      </button>
                    </th>
                    <th className={th}>
                      <button type="button" data-tid="traceability-sort-coverage" onClick={() => toggleSort('coverage')}>
                        Coverage{arrow('coverage')}
                      </button>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((req) => (
                    <tr key={req.key} data-tid={`traceability-row-${req.key}`} data-coverage={req.coverage} className="border-b border-neutral-200 hover:bg-neutral-50 align-top">
                      <td className="px-6 py-3">
                        {req.url ? (
                          <a
                            data-tid="traceability-req-link"
                            data-jira-status={req.status ?? undefined}
                            title={req.status ? `Jira status: ${req.status}` : undefined}
                            href={req.url}
                            target="_blank"
                            rel="noreferrer noopener"
                            className="font-mono text-xs font-semibold"
                            style={{ color: 'var(--color-accent)' }}
                          >
                            {req.key} ↗
                          </a>
                        ) : (
                          <span data-tid="traceability-req-key" className="font-mono text-xs font-semibold">
                            {req.key}
                          </span>
                        )}
                        <div className="text-sm text-neutral-900">{req.title ?? '—'}</div>
                        {req.unknownInJira && (
                          <div data-tid="traceability-unknown" className="text-xs text-neutral-500">
                            unknown in Jira
                          </div>
                        )}
                        {req.jira && (
                          <div data-tid="traceability-jira-written" data-outcome={req.jira.outcome ?? 'written'} className="text-xs text-neutral-700">
                            {req.jira.values.length > 0 && <span>{jiraLine(req.jira.values)}</span>}
                            {req.jira.reason && (
                              <span data-tid="traceability-jira-reason" className="block text-neutral-500">
                                {req.jira.outcome === 'would-write' || req.jira.outcome === 'would-overwrite' ? 'Not yet in Jira: ' : 'Not written: '}
                                {req.jira.reason}
                              </span>
                            )}
                          </div>
                        )}
                      </td>
                      <td className="px-6 py-3">
                        <div data-tid={`traceability-cases-${req.key}`} className="flex gap-1 flex-wrap">
                          {req.cases.map((c) => (
                            <button
                              key={`${c.repositoryId}:${c.file}`}
                              type="button"
                              data-tid="traceability-case"
                              data-case={c.caseId ?? c.file}
                              title={c.title ?? c.file}
                              onClick={() => onOpenCase?.({ repoId: c.repositoryId, file: c.file })}
                              className="px-2 py-0.5 rounded text-xs font-mono bg-neutral-100 text-neutral-700 cursor-pointer"
                            >
                              {c.caseId ?? c.file}
                            </button>
                          ))}
                          {req.cases.length === 0 && <span className="text-sm text-neutral-500">No tests linked</span>}
                        </div>
                      </td>
                      <td className="px-6 py-3">
                        <div data-tid={`traceability-results-${req.key}`} className="flex gap-1">
                          {req.cases.map((c) => {
                            const st = RESULT[c.status ?? 'UNTESTED']
                            return (
                              <span
                                key={`${c.repositoryId}:${c.file}`}
                                data-tid="traceability-result"
                                data-status={c.status ?? 'ABSENT'}
                                title={`${c.caseId ?? c.file}: ${c.status ? st.label : 'not in this run'}`}
                                style={{ width: 18, height: 18, display: 'inline-block', background: st.bg, border: `1px ${c.status ? 'solid' : 'dashed'} ${st.bd}` }}
                              />
                            )
                          })}
                          {req.cases.length === 0 && <span className="text-sm text-neutral-500">—</span>}
                        </div>
                      </td>
                      <td className="px-6 py-3">
                        <div data-tid={`traceability-defects-${req.key}`}>
                          {req.defects.map((d) => (
                            <div key={`${d.issueKey}-${d.caseId}`} data-tid="traceability-defect" data-open={d.open}>
                              {d.url ? (
                                <a href={d.url} target="_blank" rel="noreferrer noopener" className="text-sm font-medium" style={{ color: 'var(--color-accent)' }}>
                                  {d.issueKey} ↗
                                </a>
                              ) : (
                                <span className="text-sm font-medium">{d.issueKey}</span>
                              )}
                              <div className="text-xs text-neutral-500">
                                {d.status ?? 'status unknown'}
                                {d.fromRun != null ? ` · from #${d.fromRun}` : ''}
                              </div>
                            </div>
                          ))}
                          {req.defects.length === 0 && req.coverage === 'FAILING' && (
                            <span data-tid="traceability-triage" className="text-xs font-medium" style={{ color: 'var(--color-accent)' }}>
                              needs triage
                            </span>
                          )}
                          {req.defects.length === 0 && req.coverage !== 'FAILING' && <span className="text-sm text-neutral-400">None</span>}
                        </div>
                      </td>
                      <td className="px-6 py-3">
                        <span data-tid={`traceability-coverage-${req.key}`} className={`px-2 py-1 rounded-full text-xs font-medium ${COVERAGE[req.coverage].className}`}>
                          {COVERAGE[req.coverage].label}
                        </span>
                        {req.linked > 0 && req.coverage !== 'COVERED' && (
                          <div className="text-xs text-neutral-500" data-tid="traceability-progress">
                            {req.executed}/{req.linked} run
                          </div>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {rows.length === 0 && (
                <p data-tid="traceability-empty" className="px-6 py-6 text-sm text-neutral-500">
                  No requirement matches the filters.
                </p>
              )}
            </div>
          )}
        </div>
      </main>
    </div>
  )
}
