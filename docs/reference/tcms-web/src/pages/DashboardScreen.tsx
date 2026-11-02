import { useState, useEffect } from 'react'
import RepositorySyncPanel from '../components/RepositorySyncPanel'
import Header from '../components/Header'
import { User, KPI, TestRun, Workspace } from '../types'
import { testRunApi, NO_DATA_MESSAGE } from '../utils/api'
import { Dashboard } from '../types/dashboard'
import PlanDashboard from './dashboard/PlanDashboard'
import { cachedDashboard, fetchDashboard, seedDashboard } from '../utils/useDashboard'
import '../styles/dashboard.css'

interface Trend {
  value: number
  height: string
  background: string
  run: string
}

interface Activity {
  text: string
  when: string
}

interface Repository {
  url: string
  branch: string
  lastCommit: string
  parsed: string
  resultsPath: string
  schemaErrors: Array<{ file: string; msg: string }>
}

export interface DashboardScreenProps {
  user: User
  workspaces?: Workspace[]
  currentWorkspaceId?: number | null
  onSelectWorkspace?: (workspace: Workspace) => void
  _org?: unknown
  onNavigate: (screen: string) => void
  onSignOut: () => void
}

export function VvDashboard({ user, workspaces = [], currentWorkspaceId, onSelectWorkspace, onNavigate, onSignOut }: DashboardScreenProps) {
  const [orgOpen, setOrgOpen] = useState(false)
  const [kpis, setKpis] = useState<KPI[]>([])
  const [runs] = useState<TestRun[]>([])
  const [trends, setTrends] = useState<Trend[]>([])
  const [activity, setActivity] = useState<Activity[]>([])
  const [repositories, setRepositories] = useState<Repository | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const fetchData = async () => {
      try {
        setLoading(true)

        // Only the most recent loaded V&V run drives the dashboard. No sample/mock fallback.
        const vvResults = await testRunApi.latestResults()
        if (!vvResults || !vvResults.testCases || vvResults.testCases.length === 0) {
          setError(NO_DATA_MESSAGE)
          return
        }

        // KPI cards keep the design's four slots. Values come from the loaded V&V run; the open
        // defect count has no data source yet, so that card shows a neutral dash.
        const total = vvResults.totalTestCases
        const passRate = total > 0 ? Math.round((vvResults.passed / total) * 100) : 0
        const executed = vvResults.passed + vvResults.failed
        const progress = total > 0 ? Math.round((executed / total) * 100) : 0
        setKpis([
          { label: 'Test cases', value: String(total), color: 'neutral', note: `${vvResults.passedSteps}/${vvResults.totalSteps} steps passed` },
          { label: 'Run progress', value: `${progress}%`, color: 'neutral', note: `${executed} of ${total} in this run executed` },
          { label: 'Pass rate, last run', value: `${passRate}%`, color: passRate >= 80 ? 'ok' : 'warn', note: `${vvResults.failed} failed` },
          { label: 'Open defects', value: '—', color: 'neutral', note: 'Not tracked yet' },
        ])

        // Trends / activity / repositories for the loaded run ([] / null gracefully).
        const runId = vvResults.runId
        const [trendsData, activityData, reposData] = await Promise.all([
          testRunApi.trends(),
          testRunApi.activity(runId),
          testRunApi.repositories(runId),
        ])

        setTrends(trendsData)
        setActivity(activityData)
        setRepositories(reposData)
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load dashboard data')
      } finally {
        setLoading(false)
      }
    }

    fetchData()
  }, [])

  if (workspaces.length === 0) {
    return (
      <div className="dash-screen">
        <Header
          user={user}
          workspaces={workspaces}
          currentWorkspaceId={currentWorkspaceId}
          onSelectWorkspace={onSelectWorkspace}
          onToggleOrg={() => setOrgOpen(!orgOpen)}
          onNavigate={onNavigate}
          onSignOut={onSignOut}
          currentPage="dashboard"
        />
        <main className="dash-main">
          <div className="dash-empty">
            <div className="dash-empty-icon">⚠️</div>
            <h3 className="dash-empty-title">No workspace available</h3>
            <p className="dash-empty-text">No workspace is available yet. Start by creating one.</p>
          </div>
        </main>
      </div>
    )
  }

  if (loading) {
    return (
      <div className="dash-screen">
        <Header
          user={user}
          workspaces={workspaces}
          currentWorkspaceId={currentWorkspaceId}
          onSelectWorkspace={onSelectWorkspace}
          onToggleOrg={() => setOrgOpen(!orgOpen)}
          onNavigate={onNavigate}
          onSignOut={onSignOut}
          currentPage="dashboard"
        />
        <main className="dash-main dash-loading">Loading dashboard...</main>
      </div>
    )
  }

  if (error) {
    return (
      <div className="dash-screen" data-screen-label="Dashboard">
        <Header
          user={user}
          workspaces={workspaces}
          currentWorkspaceId={currentWorkspaceId}
          onSelectWorkspace={onSelectWorkspace}
          onToggleOrg={() => setOrgOpen(!orgOpen)}
          onNavigate={onNavigate}
          onSignOut={onSignOut}
          currentPage="dashboard"
        />
        {error === NO_DATA_MESSAGE ? (
          <main className="dash-main dash-loading" data-tid="dashboard-empty" role="status">
            <strong>Nothing to show yet.</strong> Your results show here after you connect a repository with test cases and record or load a run.
            <span style={{ display: 'block', marginTop: 6, fontSize: 12 }}>{NO_DATA_MESSAGE}</span>
          </main>
        ) : (
          <main className="dash-main dash-loading dash-error">Error: {error}</main>
        )}
        {/* Repository sync status does not depend on a loaded V&V run, so it stays available. */}
        <div className="dash-error-repos">
          <RepositorySyncPanel workspaces={workspaces} currentWorkspaceId={currentWorkspaceId} onNavigate={onNavigate} />
        </div>
      </div>
    )
  }

  return (
    <div className="dash-screen" data-tid="dashboard-screen" data-screen-label="Dashboard">
      <Header
        user={user}
        workspaces={workspaces}
        currentWorkspaceId={currentWorkspaceId}
        onSelectWorkspace={onSelectWorkspace}
        orgOpen={orgOpen}
        onToggleOrg={() => setOrgOpen(!orgOpen)}
        onToggleUser={() => {}}
        onSignOut={onSignOut}
        activeNav="dashboard"
        onNavigate={onNavigate}
      />

      <main className="dash-main">
        <div className="dash-head">
          <div>
            <div className="dash-eyebrow">Northwind</div>
            <h2 className="dash-title">Overview</h2>
          </div>
          <div className="dash-actions">
            <button className="btn btn-secondary" onClick={() => onNavigate('plans')}>Plans</button>
            <button className="btn btn-primary" onClick={() => onNavigate('dashboard')}>
              Continue latest run
            </button>
          </div>
        </div>

        <div className="dash-kpis" data-tid="dashboard-kpis">
          {kpis.map((kpi, i) => (
            <div key={i} className="dash-kpi" data-tid="dashboard-kpi-card">
              <div className="dash-kpi-label">{kpi.label}</div>
              <div className={`dash-kpi-value dash-kpi-value--${kpi.color}`}>{kpi.value}</div>
              <div className="dash-kpi-note">{kpi.note}</div>
            </div>
          ))}
        </div>

        <div className="dash-content">
          <div className="dash-trend" data-tid="dashboard-trend">
            <div className="dash-trend-head">
              <h5 className="dash-h5">Pass rate, last 14 runs</h5>
            </div>
            <div className="dash-chart">
              {trends.map((t, i) => (
                <div key={i} className="dash-bar">
                  <div className="dash-bar-value">{t.value}%</div>
                  <div className="dash-bar-col" style={{ height: t.height, background: t.background }}></div>
                </div>
              ))}
            </div>
            <div className="dash-chart-labels">
              {trends.map((t, i) => (
                <div key={i} className="dash-chart-label">
                  {t.run}
                </div>
              ))}
            </div>
          </div>

          <div className="dash-side">
            <RepositorySyncPanel
              workspaces={workspaces}
              currentWorkspaceId={currentWorkspaceId}
              onNavigate={onNavigate}
              fallback={
                repositories ? (
                  <div>
                    <div className="dash-muted" data-tid="dashboard-repo-fallback">
                      From loaded V&amp;V run
                    </div>
                    <div className="dash-repo-tests" data-tid="dashboard-repo-tests">
                      <div className="dash-eyebrow">Tests · cases and results</div>
                      <div className="dash-mono">{repositories.url}</div>
                      <div className="dash-repo-grid">
                        <span className="dash-muted">Branch</span>
                        <span>{repositories.branch}</span>
                        <span className="dash-muted">Last commit</span>
                        <span>{repositories.lastCommit}</span>
                        <span className="dash-muted">Parsed</span>
                        <span>{repositories.parsed}</span>
                        <span className="dash-muted">Results path</span>
                        <span className="dash-mono">{repositories.resultsPath}</span>
                      </div>
                      <div className="dash-repo-errors">
                        <div className="dash-repo-errors-head">
                          <span className="tag tag-accent">{repositories.schemaErrors.length} schema errors</span>
                          <span className="dash-muted">excluded from runs</span>
                        </div>
                        <div className="dash-error-list">
                          {repositories.schemaErrors.map((e, i) => (
                            <div key={i}>
                              {e.file} <span className="dash-error-msg">{e.msg}</span>
                            </div>
                          ))}
                        </div>
                      </div>
                    </div>
                  </div>
                ) : undefined
              }
            />
          </div>
        </div>

        <div className="dash-bottom">
          <div className="dash-runs" data-tid="dashboard-runs">
            <h5 className="dash-h5 dash-h5--flush">Runs</h5>
            <table className="table" data-tid="dashboard-runs-table">
              <thead>
                <tr>
                  <th>Run</th>
                  <th>Plan</th>
                  <th style={{ width: '34%' }}>Progress</th>
                  <th>Source</th>
                  <th>Started</th>
                </tr>
              </thead>
              <tbody>
                {runs.map((run) => (
                  <tr key={run.id} style={{ cursor: 'pointer' }}>
                    <td style={{ fontWeight: 600 }}>{run.id}</td>
                    <td>{run.plan}</td>
                    <td>
                      <div className="dash-progress">
                        <div className="dash-progress-track">
                          <div className="dash-progress-seg" style={{ flex: run.progress, background: 'var(--color-success, #10b981)' }}></div>
                          <div className="dash-progress-seg" style={{ flex: Math.max(100 - run.progress, 0), background: 'var(--color-neutral-300, #e5e7eb)' }}></div>
                        </div>
                        <span className="dash-progress-text">{run.progress}%</span>
                      </div>
                    </td>
                    <td>{run.source}</td>
                    <td className="dash-muted">{run.startedAt}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <div className="dash-activity" data-tid="dashboard-activity">
            <h5 className="dash-h5 dash-h5--flush">Activity</h5>
            {activity.map((a, i) => (
              <div key={i} className="dash-activity-item" data-tid="dashboard-activity-item">
                <div>{a.text}</div>
                <div className="dash-activity-when">{a.when}</div>
              </div>
            ))}
          </div>
        </div>
      </main>
    </div>
  )
}

/**
 * The dashboard (ORCH-026). A workspace with repositories, plans or runs gets the plan-run dashboard
 * (D-117); one with none of them keeps the view of the loaded V&V run.
 */
export default function DashboardScreen(props: DashboardScreenProps) {
  const { user, workspaces = [], currentWorkspaceId, onSelectWorkspace, onNavigate, onSignOut } = props
  const [initial, setInitial] = useState<Dashboard | null | undefined>(undefined)

  useEffect(() => {
    let alive = true
    setInitial(undefined)
    if (workspaces.length === 0) {
      setInitial(null)
      return
    }
    const cached = cachedDashboard(currentWorkspaceId)
    if (cached) {
      setInitial(cached)
      return
    }
    fetchDashboard(currentWorkspaceId)
      .then((d) => {
        if (!alive) return
        seedDashboard(currentWorkspaceId, d)
        setInitial(d)
      })
      .catch(() => alive && setInitial(null))
    return () => {
      alive = false
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentWorkspaceId, workspaces.length])

  if (initial === undefined) {
    return (
      <div className="dash-screen" data-tid="dashboard-screen" data-screen-label="Dashboard">
        <Header user={user} workspaces={workspaces} currentWorkspaceId={currentWorkspaceId} onSelectWorkspace={onSelectWorkspace} currentPage="dashboard" onNavigate={onNavigate} onSignOut={onSignOut} />
        <main className="dash-main dash-loading" data-tid="dashboard-loading">Loading dashboard...</main>
      </div>
    )
  }
  const hasPlanData = initial != null && (initial.repositories.length > 0 || initial.plans.length > 0 || initial.recentRuns.length > 0)
  if (!hasPlanData || initial == null) return <VvDashboard {...props} />
  return <PlanDashboard {...props} initial={initial} />
}
