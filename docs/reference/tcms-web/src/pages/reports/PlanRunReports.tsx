import { useEffect, useMemo, useState } from 'react'
import { Run } from '../../types/runs'
import { User, Workspace } from '../../types'
import { History, ReportProblem, RunReport, Stability } from '../../types/reports'
import { CaseStatus } from '../../types/runs'
import Header from '../../components/Header'
import { reportsApi } from '../../utils/reportsApi'
import { RunDocument } from '../../types/reports'
import TpdgDialog, { TpdgRequest } from './TpdgDialog'
import TpdgProgress from './TpdgProgress'
import TpdgDocument from './TpdgDocument'
import ShareDialog from '../../components/ShareDialog'
import { RunSharing } from '../../types/share'
import { shareApi } from '../../utils/shareApi'
import { clearOpenRun, peekOpenRun } from '../../utils/openRun'
import { rememberLayout, sessionLayout, useWorkspaceSettings } from '../../utils/useWorkspaceSettings'
import DefectCell from '../../components/DefectCell'
import Spinner from '../../components/Spinner'
import StatusBar from '../dashboard/StatusBar'
import { canAdminister, canRecord, currentRole } from '../../utils/roles'
import HeatCell from '../../components/heat/HeatCell'
import { CASE_TONE, VERDICT_COLOR } from '../../components/heat/tones'
import '../../styles/reports.css'

interface Props {
  user: User
  workspaces?: Workspace[]
  currentWorkspaceId?: number | null
  onSelectWorkspace?: (workspace: Workspace) => void
  onNavigate: (screen: string) => void
  onSignOut: () => void
  runs: Run[]
}

type View = 'summary' | 'heatmap'
type ListFilter = 'none' | 'FAILED' | 'BLOCKED' | 'FLAKY'

const CELL: Record<CaseStatus, { bg: string; label: string }> = CASE_TONE
const STABILITY_COLOR: Record<Stability, string> = { STABLE: '#2e7d32', FLAKY: '#f59e0b', FAILING: 'var(--color-accent, #d32f2f)' }
const STABILITY_LABEL: Record<Stability, string> = { STABLE: 'Stable', FLAKY: 'Flaky', FAILING: 'Failing' }

/** The design's triage tag: Flaky when the case flips, Blocked for a blocked case, else Failing. */
function triageTag(p: ReportProblem): { label: string; className: string } {
  if (p.status === 'BLOCKED') return { label: 'Blocked', className: 'tag-neutral' }
  if (p.stability === 'FLAKY') return { label: 'Flaky', className: 'tag-outline' }
  return { label: 'Failing', className: 'tag-accent' }
}

const day = (iso: string | null) => (iso ? new Date(iso).toISOString().slice(0, 10) : '—')
const arrow = (delta: number | null) => (delta == null ? '' : delta > 0 ? '↑' : delta < 0 ? '↓' : '→')
const signed = (delta: number) => (delta > 0 ? `+${delta}%` : `${delta}%`)

/** Reports over plan runs (ORCH-016): summary, history matrix and exports of one run. */
export default function PlanRunReports({ user, workspaces = [], currentWorkspaceId, onSelectWorkspace, onNavigate, onSignOut, runs }: Props) {
  const { settings, loaded: settingsLoaded } = useWorkspaceSettings(currentWorkspaceId)
  const [view, setViewState] = useState<View>(() => {
    const own = sessionLayout('reports', currentWorkspaceId)
    return own === 'heatmap' || own === 'summary' ? own : 'summary'
  })
  const setView = (v: View) => {
    setViewState(v)
    rememberLayout('reports', currentWorkspaceId, v)
  }
  useEffect(() => {
    if (settingsLoaded && sessionLayout('reports', currentWorkspaceId) == null) setViewState(settings.defaultReportLayout === 'HEATMAP' ? 'heatmap' : 'summary')
  }, [settingsLoaded, settings.defaultReportLayout, currentWorkspaceId])

  // The run the report is about: the one handed over (View report), else the latest run (D-87).
  const [runId, setRunId] = useState<number>(() => {
    const wanted = peekOpenRun()
    return runs.find((r) => r.id === wanted)?.id ?? runs[0].id
  })
  useEffect(() => clearOpenRun(), [])
  const run = runs.find((r) => r.id === runId) ?? runs[0]
  const plans = useMemo(() => {
    const seen = new Map<number, string>()
    for (const r of runs) if (!seen.has(r.planId)) seen.set(r.planId, r.planName)
    return [...seen].map(([id, name]) => ({ id, name }))
  }, [runs])
  const planRuns = runs.filter((r) => r.planId === run.planId)

  const [tpdgOpen, setTpdgOpen] = useState(false)
  const [shareOpen, setShareOpen] = useState(false)
  const [sharing, setSharing] = useState<RunSharing | null>(null)
  const loadSharing = () => {
    shareApi.sharing(run.id).then(setSharing).catch(() => setSharing(null))
  }
  useEffect(() => {
    setSharing(null)
    loadSharing()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [run.id])
  const [progress, setProgress] = useState<{ step: number; eta: number | null } | null>(null)
  const [doc, setDoc] = useState<RunDocument | null>(null)
  const [abort, setAbort] = useState<AbortController | null>(null)
  const [report, setReport] = useState<RunReport | null>(null)
  const [history, setHistory] = useState<History | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [listFilter, setListFilter] = useState<ListFilter>('none')
  const [showAll, setShowAll] = useState(false)
  const [showFailing, setShowFailing] = useState(false)
  const [suiteSort, setSuiteSort] = useState<'name' | 'rate'>('name')
  const [search, setSearch] = useState('')
  const [suiteFilter, setSuiteFilter] = useState('')
  const [sourceFilter, setSourceFilter] = useState('')

  useEffect(() => {
    let alive = true
    setReport(null)
    setError(null)
    reportsApi
      .summary(run.id)
      .then((r) => alive && setReport(r))
      .catch((e) => alive && setError(e instanceof Error ? e.message : 'Failed to load the report'))
    return () => {
      alive = false
    }
  }, [run.id])

  useEffect(() => {
    if (view !== 'heatmap') return
    let alive = true
    setHistory(null)
    reportsApi
      .history({ runId: run.id })
      .then((h) => alive && setHistory(h))
      .catch((e) => alive && setError(e instanceof Error ? e.message : 'Failed to load the history'))
    return () => {
      alive = false
    }
  }, [view, run.id, report?.gate.thresholdPercent])

  const selectPlan = (planId: number) => {
    const next = runs.find((r) => r.planId === planId)
    if (next) setRunId(next.id)
    setListFilter('none')
  }

  const fileBase = `${run.planName.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '')}_run-${run.number}_${new Date().toISOString().slice(0, 10)}`

  const exportCsv = async () => {
    try {
      const blob = await reportsApi.exportCsv(run.id, view === 'heatmap' ? 'heatmap' : 'summary')
      const url = URL.createObjectURL(blob)
      const link = document.createElement('a')
      link.href = url
      link.download = `${fileBase}${view === 'heatmap' ? '_history' : ''}.csv`
      document.body.appendChild(link)
      link.click()
      link.remove()
      URL.revokeObjectURL(url)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Could not export the report')
    }
  }

  /** Generates the Plan + Run document (ORCH-017): data first, then the print view or the CSV file. */
  const generate = async (req: TpdgRequest) => {
    setTpdgOpen(false)
    setError(null)
    const controller = new AbortController()
    setAbort(controller)
    const KEY = `tpdg:last:${currentWorkspaceId ?? ''}`
    let last: number | null = null
    try {
      last = Number(window.sessionStorage.getItem(KEY)) || null
    } catch {
      last = null
    }
    const started = Date.now()
    const eta = (step: number) => (last == null ? null : Math.max(0, Math.round((last * (3 - step)) / 3 / 1000)))
    setProgress({ step: 0, eta: eta(0) })
    try {
      if (req.format === 'csv') {
        const blob = await reportsApi.exportCsv(run.id, 'tpdg', { fromRun: req.fromRun, toRun: req.toRun })
        setProgress({ step: 2, eta: 0 })
        const url = URL.createObjectURL(blob)
        const link = document.createElement('a')
        link.href = url
        link.download = `${fileBase}.csv`
        document.body.appendChild(link)
        link.click()
        link.remove()
        URL.revokeObjectURL(url)
        return
      }
      const sections = [...(req.history ? ['history'] : []), ...(req.trace ? ['trace'] : [])]
      const data = await reportsApi.document(run.id, { fromRun: req.fromRun, toRun: req.toRun, sections, signal: controller.signal })
      setProgress({ step: 1, eta: eta(1) })
      await new Promise((r) => window.setTimeout(r, 0))
      setProgress({ step: 2, eta: eta(2) })
      setDoc(data)
      try {
        window.sessionStorage.setItem(KEY, String(Date.now() - started))
      } catch {
        // the estimate just stays unknown
      }
    } catch (e) {
      if (!(e instanceof DOMException && e.name === 'AbortError')) setError(e instanceof Error ? e.message : 'Could not generate the document')
    } finally {
      setProgress(null)
      setAbort(null)
    }
  }

  const printDocument = () => {
    const previous = document.title
    document.title = fileBase
    const restore = () => {
      document.title = previous
      window.removeEventListener('afterprint', restore)
    }
    window.addEventListener('afterprint', restore)
    window.print()
  }

  const exportPdf = () => {
    // The browser's print dialog saves a PDF; the title becomes the default file name (D-100).
    const previous = document.title
    document.title = fileBase
    const restore = () => {
      document.title = previous
      window.removeEventListener('afterprint', restore)
    }
    window.addEventListener('afterprint', restore)
    window.print()
  }

  const problems = report?.problems ?? []
  const failed = problems.filter((p) => p.status === 'FAILED')
  const filtered = (list: ReportProblem[]) =>
    list.filter((p) => {
      if (listFilter === 'FAILED' && p.status !== 'FAILED') return false
      if (listFilter === 'BLOCKED' && p.status !== 'BLOCKED') return false
      if (listFilter === 'FLAKY' && p.stability !== 'FLAKY') return false
      return true
    })

  const sources = [...new Set(problems.map((p) => (p.source ? p.source : 'MANUAL')))]
  const suiteNames = [...new Set(problems.map((p) => p.suite))].sort()
  const fullList = filtered(problems).filter((p) => {
    const q = search.trim().toLowerCase()
    if (q && !`${p.caseId ?? ''} ${p.title ?? ''}`.toLowerCase().includes(q)) return false
    if (suiteFilter && p.suite !== suiteFilter) return false
    if (sourceFilter && (p.source ?? 'MANUAL') !== sourceFilter) return false
    return true
  })
  const triage = failed.slice(0, 10)

  const suites = useMemo(() => {
    const list = [...(report?.suites ?? [])]
    if (suiteSort === 'rate') list.sort((a, b) => (b.passRate ?? -1) - (a.passRate ?? -1) || a.path.localeCompare(b.path))
    return list
  }, [report?.suites, suiteSort])

  const gate = report?.gate
  const rateText = gate ? (gate.passRate == null ? 'No data' : `${gate.passRate}%`) : '—'
  const verdictText = gate ? (gate.verdict === 'NO_DATA' ? 'NO DATA' : gate.verdict) : ''

  const defectCell = (p: ReportProblem) => (report ? <DefectCell problem={p} run={report.run} canEdit={canRecord(currentRole(workspaces, currentWorkspaceId))} onNavigate={onNavigate} canConnect={canAdminister(currentRole(workspaces, currentWorkspaceId))} /> : null)
  const since = (p: ReportProblem) => (p.failingSince ? `#${p.failingSince.number} · ${day(p.failingSince.createdAt)}` : '—')

  if (doc) {
    return (
      <div className="reports-screen" data-tid="reports-screen" data-screen-label="Reports">
        <div className="reports-noprint">
          <Header user={user} workspaces={workspaces} currentWorkspaceId={currentWorkspaceId} onSelectWorkspace={onSelectWorkspace} currentPage="reports" onNavigate={onNavigate} onSignOut={onSignOut} />
        </div>
        <TpdgDocument doc={doc} onBack={() => setDoc(null)} onPrint={printDocument} />
      </div>
    )
  }

  return (
    <div className="reports-screen" data-tid="reports-screen" data-screen-label="Reports">
      <div className="reports-noprint">
        <Header
          user={user}
          workspaces={workspaces}
          currentWorkspaceId={currentWorkspaceId}
          onSelectWorkspace={onSelectWorkspace}
          currentPage="reports"
          onNavigate={onNavigate}
          onSignOut={onSignOut}
        />
      </div>

      <main className="reports-main">
        <div className="reports-head" data-tid="reports-header">
          <div>
            <div className="reports-eyebrow" data-tid="reports-meta">
              {run.planName} · Run #{run.number} · {day(run.createdAt)} · {run.origin === 'CI' ? 'started by CI' : `started by ${run.createdBy ?? 'unknown'}`}
              {run.testsCommit ? ` · tests @ ${run.testsCommit.slice(0, 7)}` : ''}
            </div>
            <div className="reports-title-row">
              <h2 className="reports-title">Reports</h2>
              <div className="reports-selects reports-noprint">
                <label className="reports-eyebrow">
                  Plan{' '}
                  <select data-tid="reports-plan-select" value={run.planId} onChange={(e) => selectPlan(Number(e.target.value))}>
                    {plans.map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.name}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="reports-eyebrow">
                  Run{' '}
                  <select data-tid="reports-run-select" value={run.id} onChange={(e) => { setRunId(Number(e.target.value)); setListFilter('none') }}>
                    {planRuns.map((r) => (
                      <option key={r.id} value={r.id}>
                        #{r.number} · {day(r.createdAt)}
                        {r.status === 'OPEN' ? ' (open)' : ''}
                      </option>
                    ))}
                  </select>
                </label>
              </div>
            </div>
          </div>

          <div className="reports-head-actions reports-noprint">
            <div data-tid="reports-viewswitch" className="reports-seg" role="tablist" aria-label="Reports view">
              <button data-tid="reports-view-summary" type="button" role="tab" aria-selected={view === 'summary'} onClick={() => setView('summary')} className={view === 'summary' ? 'is-active' : ''}>
                Summary
              </button>
              <button data-tid="reports-view-heatmap" type="button" role="tab" aria-selected={view === 'heatmap'} onClick={() => setView('heatmap')} className={view === 'heatmap' ? 'is-active' : ''}>
                History matrix
              </button>
            </div>
            <button data-tid="reports-share" type="button" className="btn btn-secondary" onClick={() => setShareOpen(true)} title="Share this report with people outside the workspace">
              Share
              <span data-tid="reports-share-status" style={{ marginLeft: 6, fontSize: 11 }}>
                {sharing == null ? '' : sharing.visibility === 'PUBLIC' ? 'Public' : sharing.links.some((l) => l.status === 'ACTIVE') ? 'Shared' : 'Private'}
              </span>
            </button>
            <button data-tid="reports-tpdg" type="button" className="btn btn-secondary" onClick={() => setTpdgOpen(true)} title="Builds a document of the plan and its runs, for the runs you pick (TPDG)">
              Test plan document
            </button>
            <button data-tid="reports-export-pdf" type="button" className="btn btn-primary" onClick={exportPdf} title="Exports this summary: opens the print dialog; choose Save as PDF">
              Export PDF
            </button>
            <button data-tid="reports-export-csv" type="button" className="btn btn-primary" onClick={exportCsv}>
              CSV
            </button>
          </div>
        </div>

        {shareOpen && <ShareDialog runId={run.id} runLabel={`run #${run.number}`} onClose={() => setShareOpen(false)} onChanged={loadSharing} />}
        {tpdgOpen && <TpdgDialog planRuns={planRuns} run={run} onGenerate={generate} onClose={() => setTpdgOpen(false)} />}
        {progress && <TpdgProgress step={progress.step} etaSeconds={progress.eta} onCancel={() => abort?.abort()} />}
        {error && (
          <div data-tid="reports-error" className="reports-error" role="alert">
            <p>Error: {error}</p>
          </div>
        )}
        {!report && !error && (
          <div data-tid="reports-loading" className="reports-note" aria-busy="true">
            <p><Spinner label="Loading reports" /> Loading reports...</p>
          </div>
        )}

        {report && view === 'summary' && (
          <div data-tid="reports-summary">
            <div data-tid="reports-metrics" className="reports-metrics">
              <div data-tid="reports-metric-passrate" className="reports-metric reports-metric--lead">
                <div className="reports-metric-label">Pass rate</div>
                <div className="reports-metric-value reports-metric-value--lead">{rateText}</div>
                <div className="reports-metric-note reports-verdict-line">
                  <span data-tid="reports-verdict" data-verdict={gate!.verdict} style={{ color: VERDICT_COLOR[gate!.verdict], fontWeight: 700 }}>
                    {gate!.passRate != null ? `${gate!.passRate}% · ${verdictText}` : verdictText}
                  </span>
                  {gate!.provisional && <span data-tid="reports-provisional"> · in progress</span>}
                  <span data-tid="reports-gate" title={`The quality gate: the pass rate this run must reach to pass. Here it is ${gate!.thresholdPercent}%, needed to pass.`}> · gate {gate!.thresholdPercent}%</span>
                  <span data-tid="reports-metric-totaltests"> · {report.counts.total} tests</span>
                </div>
                <div data-tid="reports-delta" className="reports-metric-note reports-delta-line">
                  {report.previous == null
                    ? '— first run'
                    : report.previous.delta == null
                      ? `— no data in run #${report.previous.number}`
                      : `${arrow(report.previous.delta)} ${signed(report.previous.delta)} vs run #${report.previous.number}`}
                  {' · '}
                  <span data-tid="reports-executed" title="Executed: cases that have a result (passed, failed or blocked). Untested: cases with no result yet.">
                    {report.counts.executed} executed · {report.counts.skipped} skipped · {report.counts.untested} untested
                  </span>
                </div>
              </div>
              <button
                data-tid="reports-metric-failed"
                type="button"
                onClick={() => { setListFilter(listFilter === 'FAILED' ? 'none' : 'FAILED'); setShowFailing(true) }}
                className="reports-metric reports-metric--failed"
                aria-pressed={listFilter === 'FAILED'}
              >
                <span className="reports-metric-label">Failed</span>
                <span className="reports-metric-value reports-metric-value--failed">{report.counts.failed}</span>
                <span className="reports-metric-note">{report.counts.failed > 0 ? 'in this run' : 'none in this run'}</span>
                <span className="reports-metric-cta">Show failing tests ↓</span>
              </button>
              <button
                data-tid="reports-metric-blocked"
                type="button"
                onClick={() => { setListFilter(listFilter === 'BLOCKED' ? 'none' : 'BLOCKED'); setShowFailing(true) }}
                className="reports-metric"
                aria-pressed={listFilter === 'BLOCKED'}
              >
                <span className="reports-metric-label">Blocked</span>
                <span className="reports-metric-value">{report.counts.blocked}</span>
                <span className="reports-metric-note">in this run</span>
              </button>
              <button
                data-tid="reports-metric-flaky"
                type="button"
                onClick={() => { setListFilter(listFilter === 'FLAKY' ? 'none' : 'FLAKY'); setShowFailing(true) }}
                className="reports-metric"
                aria-pressed={listFilter === 'FLAKY'}
              >
                <span className="reports-metric-label">Flaky</span>
                <span className="reports-metric-value">{report.flakyCount}</span>
                <span className="reports-metric-note">4+ flips in the last 12 runs</span>
              </button>
            </div>

            <div data-tid="reports-lower" className="reports-lower">
            <div className="reports-failed" data-tid="reports-suites">
              <div className="reports-failed-head">
                <h5 className="reports-h5">By suite</h5>
                <div className="reports-seg reports-noprint" role="group" aria-label="Sort suites">
                  <button type="button" data-tid="reports-suite-sort-name" className={suiteSort === 'name' ? 'is-active' : ''} onClick={() => setSuiteSort('name')}>
                    By name
                  </button>
                  <button type="button" data-tid="reports-suite-sort-rate" className={suiteSort === 'rate' ? 'is-active' : ''} onClick={() => setSuiteSort('rate')}>
                    By pass rate
                  </button>
                </div>
              </div>
              <table className="table" data-tid="reports-suites-table">
                <thead>
                  <tr>
                    <th>Suite</th>
                    <th style={{ width: '44%' }}>Results</th>
                    <th>Passed</th>
                    <th>Failed</th>
                    <th>Blocked</th>
                    <th>Pass rate</th>
                  </tr>
                </thead>
                <tbody>
                  {suites.map((s) => (
                    <tr key={s.path || '(root)'} data-tid="reports-suite-row" data-suite={s.path}>
                      <td className="reports-name" style={{ paddingLeft: 16 + (suiteSort === 'name' ? s.depth * 16 : 0) }}>{s.name}</td>
                      <td>
                        <StatusBar counts={s} />
                      </td>
                      <td>{s.passed}</td>
                      <td className={s.failed > 0 ? 'reports-fail' : undefined}>{s.failed}</td>
                      <td>{s.blocked}</td>
                      <td className="reports-rate" data-tid="reports-suite-rate">{s.passRate == null ? '—' : `${s.passRate}%`}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            <div data-tid="reports-triage">
              <div className="reports-failed-head">
                <h5 className="reports-h5">
                  Failures to triage <span className="reports-h5-count">{failed.length}</span>
                </h5>
              </div>
              {failed.length === 0 ? (
                <p data-tid="reports-triage-empty" className="reports-muted">No failed cases in this run.</p>
              ) : (
                <table className="table reports-triage" data-tid="reports-triage-table">
                  <thead>
                    <tr>
                      <th>ID</th>
                      <th>Title</th>
                      <th>Status</th>
                      <th>Defect</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(showAll ? failed : triage).map((p) => {
                      const tag = triageTag(p)
                      return (
                        <tr key={p.runCaseId} data-tid="reports-triage-row" data-case={p.caseId ?? ''}>
                          <td className="reports-mono reports-strong">{p.caseId ?? '—'}</td>
                          <td>
                            <div>{p.title}</div>
                            <div className="reports-triage-note">
                              {[p.suite || '(root)', p.notes].filter(Boolean).join(' · ')}
                            </div>
                          </td>
                          <td>
                            <span className={`tag ${tag.className}`}>{tag.label}</span>
                          </td>
                          <td className="reports-small">{defectCell(p)}</td>
                        </tr>
                      )
                    })}
                  </tbody>
                </table>
              )}
              {failed.length > 10 && !showAll && (
                <button type="button" data-tid="reports-triage-showall" className="btn btn-secondary reports-noprint" onClick={() => setShowAll(true)}>
                  Show all {failed.length}
                </button>
              )}
            </div>
            </div>

            <div className="reports-noprint" style={{ margin: '8px 0' }}>
              <button type="button" data-tid="reports-failing-toggle" className="btn btn-secondary" aria-expanded={showFailing} onClick={() => setShowFailing((v) => !v)}>
                {showFailing ? 'Hide failing tests' : 'Show failing tests'}
              </button>
            </div>

            {showFailing && (
              <div data-tid="reports-failed-table" className="reports-failed">
                <div className="reports-failed-head">
                  <h5 className="reports-h5">
                    Failing tests{listFilter === 'BLOCKED' ? ' (blocked)' : listFilter === 'FLAKY' ? ' (flaky)' : ''} <span className="reports-h5-count">{fullList.length}</span>
                  </h5>
                  <button type="button" className="btn btn-ghost reports-noprint" data-tid="reports-failing-close" onClick={() => { setShowFailing(false); setListFilter('none') }}>
                    Close
                  </button>
                </div>
                <div className="reports-noprint" style={{ display: 'flex', gap: 8, flexWrap: 'wrap', marginBottom: 8 }}>
                  <input data-tid="reports-failing-search" className="input" placeholder="Search ID or title" value={search} onChange={(e) => setSearch(e.target.value)} />
                  <select data-tid="reports-failing-suite" value={suiteFilter} onChange={(e) => setSuiteFilter(e.target.value)} aria-label="Suite">
                    <option value="">All suites</option>
                    {suiteNames.map((n) => (
                      <option key={n} value={n}>
                        {n || '(root)'}
                      </option>
                    ))}
                  </select>
                  <select data-tid="reports-failing-source" value={sourceFilter} onChange={(e) => setSourceFilter(e.target.value)} aria-label="Source">
                    <option value="">All sources</option>
                    {sources.map((n) => (
                      <option key={n} value={n}>
                        {n === 'CI' ? 'CI' : 'Manual'}
                      </option>
                    ))}
                  </select>
                </div>
                <table className="table">
                  <thead>
                    <tr>
                      <th>ID</th>
                      <th>Title</th>
                      <th>Suite</th>
                      <th>Source</th>
                      <th>Failing since</th>
                      <th>Defect</th>
                    </tr>
                  </thead>
                  <tbody>
                    {fullList.map((p) => (
                      <tr key={p.runCaseId} data-tid={`reports-failed-row-${p.caseId ?? p.runCaseId}`} data-status={p.status}>
                        <td className="reports-mono reports-strong">{p.caseId ?? '—'}</td>
                        <td>{p.title}</td>
                        <td className="reports-small">{p.suite || '(root)'}</td>
                        <td className="reports-small reports-mono">{p.source === 'CI' ? 'ci' : 'manual'}</td>
                        <td className="reports-small" data-tid="reports-since">{p.status === 'FAILED' ? since(p) : '—'}</td>
                        <td className="reports-small">{defectCell(p)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                {fullList.length === 0 && (
                  <p data-tid="reports-failing-empty" className="reports-muted">
                    Nothing matches.
                  </p>
                )}
              </div>
            )}
          </div>
        )}

        {view === 'heatmap' && !history && !error && (
          <div data-tid="reports-loading" className="reports-note" aria-busy="true">
            <p><Spinner label="Loading the history matrix" /> Loading the history matrix...</p>
          </div>
        )}

        {view === 'heatmap' && history && (
          <div data-tid="reports-heatmap" className="space-y-4">
            <div className="text-sm text-neutral-700">
              Result of each case in the plan's last {history.runs.length} runs, newest on the right. Run columns are coloured by the quality gate ({history.gate.thresholdPercent}%); cases that flip between pass and fail 4+ times are flaky.
            </div>
            <div className="bg-white rounded-lg border border-neutral-200 p-4 overflow-auto">
              <div data-tid="reports-heatmap-grid" style={{ minWidth: 520 + history.runs.length * 36 }}>
                <div
                  className="grid items-end pb-1.5 border-b-2 border-neutral-200 text-neutral-700"
                  style={{ gridTemplateColumns: `96px minmax(200px,1.4fr) repeat(${history.runs.length},minmax(32px,1fr)) 96px`, gap: 3, fontSize: 11 }}
                >
                  <span>Case</span>
                  <span>Title</span>
                  {history.runs.map((r) => (
                    <span
                      key={r.id}
                      data-tid="reports-heatmap-run"
                      data-verdict={r.verdict}
                      title={`Run #${r.number} · ${day(r.createdAt)} · ${r.passRate == null ? 'no data' : `${r.passRate}%`}${r.provisional ? ' · in progress' : ''}`}
                      style={{ textAlign: 'center', padding: '2px 0', color: '#fff', fontWeight: 600, background: VERDICT_COLOR[r.verdict] }}
                    >
                      #{r.number}
                      <span style={{ display: 'block', fontSize: 9, fontWeight: 400 }}>{day(r.createdAt).slice(5)}</span>
                    </span>
                  ))}
                  <span style={{ paddingLeft: 8 }}>Stability</span>
                </div>

                {groupBySuite(history).map(([suite, rows]) => (
                  <div key={suite || '(root)'} data-tid="reports-heatmap-group" data-suite={suite}>
                    <div className="reports-eyebrow" style={{ padding: '8px 0 2px' }}>{suite || '(root)'}</div>
                    {rows.map((row) => (
                      <div
                        key={row.key}
                        data-tid={`reports-heatmap-row-${row.key}`}
                        className="grid items-center border-b border-neutral-200"
                        style={{ gridTemplateColumns: `96px minmax(200px,1.4fr) repeat(${history.runs.length},minmax(32px,1fr)) 96px`, gap: 3, padding: '3px 0' }}
                      >
                        <span className="font-semibold text-neutral-900 reports-mono" style={{ fontSize: 12 }}>{row.caseId ?? row.key}</span>
                        <span style={{ fontSize: 13, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{row.title}</span>
                        {row.cells.map((cell, i) => (
                          <HeatCell
                            key={i}
                            data-tid="reports-heatmap-cell"
                            data-status={cell ?? 'ABSENT'}
                            title={`#${history.runs[i].number}: ${cell ? CELL[cell].label : 'not in run'}`}
                            bg={cell ? CELL[cell].bg : null}
                          />
                        ))}
                        <span style={{ paddingLeft: 8 }}>
                          <span data-tid="reports-stability" data-stability={row.stability} className="inline-block rounded-full px-2 py-0.5 text-xs font-semibold text-white" style={{ background: STABILITY_COLOR[row.stability] }}>
                            {STABILITY_LABEL[row.stability]}
                          </span>
                        </span>
                      </div>
                    ))}
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
      </main>
    </div>
  )
}

function groupBySuite(history: History): [string, History['rows']][] {
  const groups = new Map<string, History['rows']>()
  for (const row of history.rows) {
    const list = groups.get(row.suite) ?? []
    list.push(row)
    groups.set(row.suite, list)
  }
  return [...groups].sort((a, b) => a[0].localeCompare(b[0]))
}
