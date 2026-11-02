import { useEffect, useState } from 'react'
import { User, Workspace } from '../types'
import Header from '../components/Header'
import Spinner from '../components/Spinner'
import { testRunApi, NO_DATA_MESSAGE } from '../utils/api'
import { runsApi } from '../utils/runsApi'
import { useProduct } from '../utils/product'
import PlanRunReports from './reports/PlanRunReports'
import { Run } from '../types/runs'
import '../styles/reports.css'

type ReportsView = 'summary' | 'heatmap'

interface FailingTest {
  id: string
  title: string
  suite: string
  source: string
  since: string
  defect: string
}

interface HeatmapRow {
  id: string
  title: string
  cells: ('pass' | 'fail' | 'flaky')[]
  stability: string
  stabilityKind: 'stable' | 'flaky' | 'watch'
}

export interface ReportsScreenProps {
  user: User
  workspaces?: Workspace[]
  currentWorkspaceId?: number | null
  onSelectWorkspace?: (workspace: Workspace) => void
  onNavigate: (screen: string) => void
  onSignOut: () => void
}

const CELL_COLORS: Record<'pass' | 'fail' | 'flaky', string> = {
  pass: '#2e7d32',
  fail: 'var(--color-accent, #d32f2f)',
  flaky: '#f59e0b',
}

const STABILITY_COLORS: Record<HeatmapRow['stabilityKind'], string> = {
  stable: '#2e7d32',
  flaky: '#f59e0b',
  watch: 'var(--color-accent, #d32f2f)',
}

export function VvReports({ user, workspaces = [], currentWorkspaceId, onSelectWorkspace, onNavigate, onSignOut }: ReportsScreenProps) {
  const [failingTests, setFailingTests] = useState<FailingTest[]>([])
  const [heatmapRows, setHeatmapRows] = useState<HeatmapRow[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [view, setView] = useState<ReportsView>('summary')
  const [showFailed, setShowFailed] = useState(false)
  const [metrics, setMetrics] = useState({
    passRate: '—',
    failed: 0,
    totalTests: 0,
    coverage: '—',
  })

  useEffect(() => {
    const fetchReports = async () => {
      try {
        const vvResults = await testRunApi.latestResults()

        // No run loaded (or an empty run) → show the shared no-data message, no mock fallback.
        if (!vvResults || !vvResults.testCases || vvResults.testCases.length === 0) {
          setError(NO_DATA_MESSAGE)
          return
        }

        const passRate = vvResults.totalTestCases > 0
          ? Math.round((vvResults.passed / vvResults.totalTestCases) * 100)
          : 0
        setMetrics({
          passRate: `${passRate}%`,
          failed: vvResults.failed,
          totalTests: vvResults.totalTestCases,
          coverage: `${vvResults.passedSteps}/${vvResults.totalSteps}`,
        })

        // Failing tests and heatmap from the backend's derived views.
        const [failingData, heatmapData] = await Promise.all([
          testRunApi.failing(vvResults.runId),
          testRunApi.heatmap(),
        ])

        setFailingTests(
          (failingData ?? []).map((f, i) => ({
            id: f.id || `TC-${i}`,
            title: f.title || `Test ${i}`,
            suite: f.suite || 'Unknown',
            source: f.source || '',
            since: f.since || '',
            defect: f.defect || '',
          })),
        )

        setHeatmapRows(
          (heatmapData ?? []).map((h, i) => {
            const stabilityKind = (h.stability ?? 'stable') as 'stable' | 'flaky' | 'watch'
            const label = stabilityKind.charAt(0).toUpperCase() + stabilityKind.slice(1)
            return {
              id: h.id || `TC-${i}`,
              title: h.title || 'Test Case',
              cells: Array.isArray(h.cells) ? h.cells : [],
              stability: label,
              stabilityKind,
            }
          }),
        )
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load reports')
      } finally {
        setLoading(false)
      }
    }
    fetchReports()
  }, [])

  const RUN_LABELS = Array.from({ length: heatmapRows[0]?.cells.length ?? 12 }, (_, i) => `Run ${i + 1}`)

  return (
    <div className="reports-screen" data-tid="reports-screen" data-screen-label="Reports">
      <Header
        user={user}
        workspaces={workspaces}
        currentWorkspaceId={currentWorkspaceId}
        onSelectWorkspace={onSelectWorkspace}
        currentPage="reports"
        onNavigate={onNavigate}
        onSignOut={onSignOut}
      />

      <main className="reports-main">
        <div className="reports-head" data-tid="reports-header">
          <div>
            <div className="reports-eyebrow" data-tid="reports-eyebrow">
              {RUN_LABELS.length > 0 && heatmapRows.length > 0 ? `Last ${RUN_LABELS.length} runs, oldest first · read from /results` : 'Read from /results'}
            </div>
            <h2 className="reports-title">Reports</h2>
          </div>

          <div className="reports-head-actions">
            <div
              data-tid="reports-viewswitch"
              className="reports-seg"
              role="tablist"
              aria-label="Reports view"
            >
              <button
                data-tid="reports-view-summary"
                type="button"
                role="tab"
                aria-selected={view === 'summary'}
                onClick={() => setView('summary')}
                className={view === 'summary' ? 'is-active' : ''}
              >
                Summary
              </button>
              <button
                data-tid="reports-view-heatmap"
                type="button"
                role="tab"
                aria-selected={view === 'heatmap'}
                onClick={() => setView('heatmap')}
                className={view === 'heatmap' ? 'is-active' : ''}
              >
                Heatmap
              </button>
            </div>
            {/* Export (ORCH-016.11 builds the export itself): primary buttons as in the design, off until then. */}
            <button
              data-tid="reports-export-pdf"
              type="button"
              className="btn btn-primary"
              disabled
              title="Export arrives with report export (ORCH-016.11)"
            >
              Export PDF
            </button>
            <button
              data-tid="reports-export-csv"
              type="button"
              className="btn btn-primary"
              disabled
              title="Export arrives with report export (ORCH-016.11)"
            >
              CSV
            </button>
          </div>
        </div>

        {view === 'summary' && (
          <div data-tid="reports-summary">
            <div data-tid="reports-metrics" className="reports-metrics">
              <div data-tid="reports-metric-passrate" className="reports-metric reports-metric--lead">
                <div className="reports-metric-label">Pass rate</div>
                <div className="reports-metric-value reports-metric-value--lead">{metrics.passRate}</div>
                <div className="reports-metric-note">last run</div>
              </div>
              <button
                data-tid="reports-metric-failed"
                type="button"
                onClick={() => setShowFailed((v) => !v)}
                className="reports-metric reports-metric--failed"
                aria-expanded={showFailed}
              >
                <span className="reports-metric-label">Failed</span>
                <span className="reports-metric-value reports-metric-value--failed">{metrics.failed}</span>
                <span className="reports-metric-note">{metrics.failed > 0 ? 'in the loaded run' : 'none in the loaded run'}</span>
                <span className="reports-metric-cta">{showFailed ? 'Hide failing tests ↑' : 'Show failing tests ↓'}</span>
              </button>
              <div data-tid="reports-metric-totaltests" className="reports-metric">
                <div className="reports-metric-label">Total tests</div>
                <div className="reports-metric-value">{metrics.totalTests}</div>
                <div className="reports-metric-note">test cases</div>
              </div>
              <div data-tid="reports-metric-coverage" className="reports-metric">
                <div className="reports-metric-label">Coverage</div>
                <div className="reports-metric-value">{metrics.coverage}</div>
                <div className="reports-metric-note">steps passed</div>
              </div>
            </div>

            {showFailed && (
              <div data-tid="reports-failed-table" className="reports-failed">
                <div className="reports-failed-head">
                  <h5 className="reports-h5">
                    Failing tests, loaded run <span className="reports-h5-count">{failingTests.length}</span>
                  </h5>
                  <button type="button" className="btn btn-ghost" onClick={() => setShowFailed(false)}>
                    Close
                  </button>
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
                    {failingTests.map((f) => (
                      <tr key={f.id} data-tid={`reports-failed-row-${f.id}`}>
                        <td className="reports-mono reports-strong">{f.id}</td>
                        <td>{f.title}</td>
                        <td className="reports-small">{f.suite}</td>
                        <td className="reports-small reports-mono">{f.source}</td>
                        <td className="reports-small">{f.since}</td>
                        <td className="reports-small">
                          {f.defect === '—' || f.defect === '' ? <span className="reports-muted">—</span> : <a href="#" className="reports-accent-link">{f.defect}</a>}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        )}

        {loading && (
          <div data-tid="reports-loading" className="reports-note" aria-busy="true">
            <p><Spinner label="Loading reports" /> Loading reports...</p>
          </div>
        )}

        {error && (
          <div data-tid="reports-error" className="reports-error">
            <p>Error: {error}</p>
          </div>
        )}

        {view === 'heatmap' && (
          <div data-tid="reports-heatmap" className="space-y-4">
            <div className="flex justify-between items-center flex-wrap gap-3">
              <div className="text-sm text-neutral-700">
                Result history per case. Cases that flip between pass and fail 4+ times are marked flaky.
              </div>
              <div className="flex gap-4 text-xs">
                <span className="inline-flex items-center gap-1.5">
                  <span className="inline-block w-3 h-3 rounded-sm" style={{ background: CELL_COLORS.pass }} /> Pass
                </span>
                <span className="inline-flex items-center gap-1.5">
                  <span className="inline-block w-3 h-3 rounded-sm" style={{ background: CELL_COLORS.fail }} /> Fail
                </span>
                <span className="inline-flex items-center gap-1.5">
                  <span className="inline-block w-3 h-3 rounded-sm" style={{ background: CELL_COLORS.flaky }} /> Flaky
                </span>
              </div>
            </div>

            <div className="bg-white rounded-lg border border-neutral-200 p-4 overflow-auto">
              <div data-tid="reports-heatmap-grid" style={{ minWidth: '860px' }}>
                <div
                  className="grid items-end pb-1.5 border-b-2 border-neutral-200 text-neutral-700"
                  style={{ gridTemplateColumns: '96px minmax(220px,1.4fr) repeat(12,minmax(28px,1fr)) 96px', gap: '3px', fontSize: '11px', letterSpacing: '0.06em', textTransform: 'uppercase' }}
                >
                  <span>Case</span>
                  <span>Title</span>
                  {RUN_LABELS.map((label) => (
                    <span key={label} className="font-semibold" style={{ letterSpacing: 0, textTransform: 'none' }}>{label}</span>
                  ))}
                  <span style={{ paddingLeft: '8px' }}>Stability</span>
                </div>

                {heatmapRows.map((row) => (
                  <div
                    key={row.id}
                    data-tid={`reports-heatmap-row-${row.id}`}
                    className="grid items-center border-b border-neutral-200"
                    style={{ gridTemplateColumns: '96px minmax(220px,1.4fr) repeat(12,minmax(28px,1fr)) 96px', gap: '3px', padding: '3px 0' }}
                  >
                    <span className="font-semibold text-neutral-900" style={{ fontFamily: 'var(--font-mono)', fontSize: '12px' }}>{row.id}</span>
                    <span className="text-neutral-900" style={{ fontSize: '13px', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{row.title}</span>
                    {row.cells.map((cell, i) => (
                      <div key={i} title={`${RUN_LABELS[i]}: ${cell}`} style={{ height: '26px', borderRadius: '2px', background: CELL_COLORS[cell] }} />
                    ))}
                    <span style={{ paddingLeft: '8px' }}>
                      <span
                        className="inline-block rounded-full px-2 py-0.5 text-xs font-semibold text-white"
                        style={{ background: STABILITY_COLORS[row.stabilityKind] }}
                      >
                        {row.stability}
                      </span>
                    </span>
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

/**
 * Reports (ORCH-016). A workspace with plan runs gets the reports over those runs (D-86); one with
 * none keeps the view of the loaded V&V run, with its banner.
 */
export default function ReportsScreen(props: ReportsScreenProps) {
  const { user, workspaces = [], currentWorkspaceId, onSelectWorkspace, onNavigate, onSignOut } = props
  const [runs, setRuns] = useState<Run[] | null>(null)
  const { productId } = useProduct()

  useEffect(() => {
    let alive = true
    setRuns(null)
    runsApi
      .list({ repositoryId: productId })
      .then((list) => alive && setRuns(list))
      .catch(() => alive && setRuns([]))
    return () => {
      alive = false
    }
  }, [currentWorkspaceId, productId])

  if (runs == null) {
    return (
      <div className="reports-screen" data-tid="reports-screen" data-screen-label="Reports">
        <Header
          user={user}
          workspaces={workspaces}
          currentWorkspaceId={currentWorkspaceId}
          onSelectWorkspace={onSelectWorkspace}
          currentPage="reports"
          onNavigate={onNavigate}
          onSignOut={onSignOut}
        />
        <main className="reports-main">
          <div data-tid="reports-loading" className="reports-note" aria-busy="true">
            <p><Spinner label="Loading reports" /> Loading reports...</p>
          </div>
        </main>
      </div>
    )
  }
  if (runs.length === 0) return <VvReports {...props} />
  return <PlanRunReports {...props} runs={runs} />
}
