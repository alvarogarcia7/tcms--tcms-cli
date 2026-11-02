import { useEffect, useMemo, useRef, useState } from 'react'
import { requestSettingsSection } from '../../utils/openSettings'
import Header from '../../components/Header'
import RepositorySyncPanel from '../../components/RepositorySyncPanel'
import { useRepositorySync } from '../../utils/repositorySync'
import { User, Workspace } from '../../types'
import { ActivityItem, Dashboard, DashboardPlan } from '../../types/dashboard'
import { dashboardApi } from '../../utils/dashboardApi'
import { DASHBOARD_TTL_MS, useDashboard } from '../../utils/useDashboard'
import { canManage, currentRole } from '../../utils/roles'
import { useProduct } from '../../utils/product'
import { requestOpenPlan } from '../../utils/openPlan'
import { requestOpenRun } from '../../utils/openRun'
import StatusBar from './StatusBar'
import '../../styles/dashboard.css'

interface Props {
  user: User
  workspaces?: Workspace[]
  currentWorkspaceId?: number | null
  onSelectWorkspace?: (workspace: Workspace) => void
  onNavigate: (screen: string) => void
  onSignOut: () => void
  /** The dashboard that was already fetched to choose between this view and the V&V one. */
  initial: Dashboard
}

type SectionKey = 'kpis' | 'plans' | 'runs' | 'activity' | 'repositories'
const SECTIONS: { key: SectionKey; label: string }[] = [
  { key: 'kpis', label: 'Key figures' },
  { key: 'plans', label: 'Test plans' },
  { key: 'runs', label: 'Latest runs' },
  { key: 'activity', label: 'Activity' },
  { key: 'repositories', label: 'Repositories' },
]

const ACTION_TEXT: Record<string, string> = {
  'plan.create': 'created the plan',
  'plan.archive': 'archived the plan',
  'run.create': 'started a run of',
  'run.close': 'closed a run of',
  'results.commit': 'committed the results of',
  'results.ingest': 'posted CI results to',
  'repo.sync': 'synced the repository',
  'defect.create': 'created the defect',
  'defect.link': 'linked the defect',
}

function describe(item: ActivityItem): string {
  const verb = ACTION_TEXT[item.action] ?? item.action
  const what = item.details.planName ?? item.details.name ?? item.details.issueKey ?? item.target ?? ''
  const run = item.details.runNumber ? ` (run #${item.details.runNumber})` : ''
  return `${item.actor} ${verb} ${what}${run}`.trim()
}

const dayStart = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime()
function dateGroup(iso: string): string {
  const days = Math.round((dayStart(new Date()) - dayStart(new Date(iso))) / 86_400_000)
  return days <= 0 ? 'Today' : days === 1 ? 'Yesterday' : days < 7 ? 'This week' : 'Older'
}

/** The dashboard of a workspace with repositories, plans or runs (ORCH-026, D-117). */
export default function PlanDashboard({ user, workspaces = [], currentWorkspaceId, onSelectWorkspace, onNavigate, onSignOut, initial }: Props) {
  const isViewer = !canManage(currentRole(workspaces, currentWorkspaceId))
  const workspaceName = workspaces.find((w) => w.id === currentWorkspaceId)?.name ?? ''
  // The product filter is the header's product selection (ORCH-020.08): one state, two controls.
  const { productId: product, select: setProduct } = useProduct()
  const { data: fetched, loading, error, fetchedAt, refresh, patch } = useDashboard(currentWorkspaceId, product)
  const data = fetched ?? (product == null ? initial : null)
  const [more, setMore] = useState<ActivityItem[]>([])
  const [nextBefore, setNextBefore] = useState<number | null | undefined>(undefined)
  const [customizing, setCustomizing] = useState(false)
  const [note, setNote] = useState<string | null>(null)

  useEffect(() => {
    setMore([])
    setNextBefore(undefined)
  }, [fetchedAt])

  // A sync that read a new commit (Sync now, the header button or the background check) changes the
  // last commit and the counts shown here: reload the dashboard when the shared state says so.
  const sync = useRepositorySync()
  const commits = sync.repos.map((r) => `${r.id}:${r.lastSyncCommit ?? ''}`).join('|')
  const seenCommits = useRef<string | null>(null)
  useEffect(() => {
    if (!sync.loaded) return
    if (seenCommits.current !== null && seenCommits.current !== commits) refresh()
    seenCommits.current = commits
  }, [commits, sync.loaded, refresh])

  const hidden = new Set(data?.hiddenSections ?? [])
  const activity = useMemo(() => [...(data?.activity ?? []), ...more], [data, more])
  const nextCursor = nextBefore === undefined ? data?.nextActivityBefore ?? null : nextBefore

  const showMore = async () => {
    if (nextCursor == null) return
    const page = await dashboardApi.activity(nextCursor, 20)
    setMore((m) => [...m, ...page.items])
    setNextBefore(page.nextBefore)
  }

  const savePrefs = async (change: (p: { favoritePlanIds: number[]; hiddenSections: string[] }) => { favoritePlanIds: number[]; hiddenSections: string[] }) => {
    try {
      const current = await dashboardApi.preferences()
      const saved = await dashboardApi.savePreferences(change(current))
      patch((d) => ({
        ...d,
        hiddenSections: saved.hiddenSections,
        plans: d.plans
          .map((p) => ({ ...p, favorite: saved.favoritePlanIds.includes(p.id) }))
          .sort((a, b) => Number(b.favorite) - Number(a.favorite) || a.name.localeCompare(b.name)),
      }))
      setNote(null)
      return true
    } catch (e) {
      setNote(e instanceof Error ? e.message : 'Could not save your preferences')
      return false
    }
  }

  const toggleFavorite = (plan: DashboardPlan) =>
    savePrefs((p) => ({
      ...p,
      favoritePlanIds: plan.favorite ? p.favoritePlanIds.filter((id) => id !== plan.id) : [...p.favoritePlanIds, plan.id],
    }))

  const toggleSection = (key: SectionKey) => {
    // Shown at once, then saved; a failed save puts it back.
    const before = data?.hiddenSections ?? []
    const after = before.includes(key) ? before.filter((k) => k !== key) : [...before, key]
    patch((d) => ({ ...d, hiddenSections: after }))
    void savePrefs((p) => ({ ...p, hiddenSections: after })).then((ok) => {
      if (!ok) patch((d) => ({ ...d, hiddenSections: before }))
    })
  }

  const planAction = (plan: DashboardPlan) => {
    if (plan.action.startsWith('Continue')) {
      if (plan.runId != null) requestOpenRun(plan.runId)
      onNavigate('runs')
    } else if (plan.action === 'View report') {
      if (plan.runId != null) requestOpenRun(plan.runId)
      onNavigate('reports')
    } else {
      requestOpenPlan(plan.id)
      onNavigate('plans')
    }
  }

  const header = (
    <Header user={user} workspaces={workspaces} currentWorkspaceId={currentWorkspaceId} onSelectWorkspace={onSelectWorkspace} activeNav="dashboard" currentPage="dashboard" onNavigate={onNavigate} onSignOut={onSignOut} />
  )

  if (!data) {
    return (
      <div className="dash-screen" data-tid="dashboard-screen" data-screen-label="Dashboard">
        {header}
        <main className="dash-main dash-loading" data-tid={error ? 'dashboard-error' : 'dashboard-loading'}>
          {error ? `Error: ${error}` : 'Loading dashboard...'}
        </main>
      </div>
    )
  }

  const lastRun = data.recentRuns[0]
  const executed = data.recentRuns.filter((r) => r.status === 'OPEN').reduce((n, r) => n + r.tally.total - r.tally.untested, 0)
  const openTotal = data.recentRuns.filter((r) => r.status === 'OPEN').reduce((n, r) => n + r.tally.total, 0)
  // The design's label reads "RUN #118 PROGRESS": the open run's number is data, shown in the label as
  // the design shows it. The measurement matches the label without its number (D-182).
  const openRunNumber = data.recentRuns.find((r) => r.status === 'OPEN')?.number ?? null
  const testCases = data.repositories.filter((r) => r.kind === 'TEST').reduce((n, r) => n + (r.caseCount ?? 0), 0)
  const checklist = [
    { done: data.onboarding.repositoryConnected, label: 'Connect a test repository', to: 'settings', tid: 'connect' },
    { done: data.onboarding.synced, label: 'Sync it to read your cases', to: 'settings', tid: 'sync' },
    { done: data.onboarding.planCreated, label: 'Create your first test plan', to: 'plans', tid: 'plan' },
    { done: data.onboarding.runStarted, label: 'Start a run of a plan', to: 'plans', tid: 'run' },
  ]
  const nextStep = checklist.find((s) => !s.done)
  const groups = ['Today', 'Yesterday', 'This week', 'Older']

  // The design's chart: one bar per run of the last 14, the latest in the accent colour. The bars
  // start at 80% (as the design's axis), so the differences between runs are visible.
  const trends = (data.trend ?? []).map((t, i, all) => {
    const rate = t.passRate ?? 0
    return {
      value: rate,
      height: `${Math.max(0, Math.min(100, (rate - 80) * 5))}%`,
      background: i === all.length - 1 ? 'var(--color-accent, #d32f2f)' : 'var(--color-text)',
      run: t.number != null ? `#${t.number}` : '',
    }
  })

  return (
    <div className="dash-screen" data-tid="dashboard-screen" data-screen-label="Dashboard">
      {header}

      <main className="dash-main">
        <div className="dash-head">
          <div>
            <div className="dash-eyebrow">{workspaceName}</div>
            <h2 className="dash-title">Overview</h2>
          </div>
          <div className="dash-actions">
            {data.repositories.filter((r) => r.kind === 'TEST').length > 1 && (
              <select data-tid="dashboard-product" aria-label="Product" value={product ?? ''} onChange={(e) => setProduct(e.target.value ? Number(e.target.value) : null)}>
                <option value="">All products</option>
                {data.repositories.filter((r) => r.kind === 'TEST').map((r) => (
                  <option key={r.id} value={r.id}>{r.name}</option>
                ))}
              </select>
            )}
            <button type="button" className="btn btn-secondary" onClick={() => onNavigate('plans')}>Plans</button>
            <button type="button" className="btn btn-secondary" data-tid="dashboard-refresh"disabled={loading} onClick={() => void refresh()} title={fetchedAt ? `Loaded ${new Date(fetchedAt).toLocaleTimeString()}; cached for ${DASHBOARD_TTL_MS / 60000} minutes` : undefined}>
              {loading ? 'Refreshing…' : 'Refresh'}
            </button>
            <button type="button" className="btn btn-secondary" data-tid="dashboard-customize" aria-expanded={customizing} onClick={() => setCustomizing((v) => !v)}>
              Customize
            </button>
          </div>
        </div>

        {customizing && (
          <div data-tid="dashboard-customize-panel" className="dash-repo-tests" style={{ marginBottom: 12 }}>
            <div className="dash-eyebrow">Show these sections</div>
            {SECTIONS.map((s) => (
              <label key={s.key} style={{ display: 'block' }}>
                <input data-tid={`dashboard-section-${s.key}`} type="checkbox" checked={!hidden.has(s.key)} onChange={() => void toggleSection(s.key)} /> {s.label}
              </label>
            ))}
          </div>
        )}
        {note && <div data-tid="dashboard-note" role="alert" style={{ color: '#8a1f17', marginBottom: 8 }}>{note}</div>}

        {nextStep && (
          <div data-tid="dashboard-onboarding" className="dash-repo-tests" style={{ marginBottom: 12 }}>
            <div className="dash-eyebrow">Get started</div>
            <ol style={{ margin: '6px 0', paddingLeft: 20 }}>
              {checklist.map((s) => (
                <li key={s.tid} data-tid={`dashboard-step-${s.tid}`} data-done={s.done} style={{ textDecoration: s.done ? 'line-through' : undefined, opacity: s.done ? 0.6 : 1 }}>
                  {s.label}
                </li>
              ))}
            </ol>
            {isViewer ? (
              <span className="dash-muted">Ask an owner or workspace admin to set this workspace up.</span>
            ) : (
              <button type="button" className="btn btn-primary" data-tid="dashboard-next-step" onClick={() => { if (nextStep.to === 'settings') requestSettingsSection('repositories'); onNavigate(nextStep.to) }}>
                {nextStep.label}
              </button>
            )}
          </div>
        )}

        {!hidden.has('kpis') && (
          <div className="dash-kpis" data-tid="dashboard-kpis">
            <div className="dash-kpi" data-tid="dashboard-kpi-card">
              <div className="dash-kpi-label">Test cases</div>
              <div className="dash-kpi-value dash-kpi-value--neutral">{testCases}</div>
              <div className="dash-kpi-note">in the synced test repositories</div>
            </div>
            <div className="dash-kpi" data-tid="dashboard-kpi-card">
              <div className="dash-kpi-label">{openRunNumber != null ? `Run #${openRunNumber} progress` : 'Run progress'}</div>
              <div className="dash-kpi-value dash-kpi-value--neutral">{openTotal > 0 ? `${Math.round((executed / openTotal) * 100)}%` : '—'}</div>
              <div className="dash-kpi-note">{openTotal > 0 ? `${executed} of ${openTotal} cases executed` : 'no run in progress'}</div>
            </div>
            <div className="dash-kpi" data-tid="dashboard-kpi-card">
              <div className="dash-kpi-label">Pass rate, last run</div>
              <div className={`dash-kpi-value dash-kpi-value--${lastRun?.verdict === 'PASSED' ? 'ok' : lastRun?.verdict === 'FAILED' ? 'warn' : 'neutral'}`} data-tid="dashboard-passrate">
                {lastRun?.passRate == null ? '—' : `${lastRun.passRate}%`}
              </div>
              <div className="dash-kpi-note">{lastRun ? `run #${lastRun.number} · ${lastRun.tally.passed} passed · ${lastRun.tally.failed} failed` : 'no run yet'}</div>
            </div>
            <div className="dash-kpi" data-tid="dashboard-kpi-card">
              <div className="dash-kpi-label">Open defects</div>
              <div className="dash-kpi-value dash-kpi-value--neutral" data-tid="dashboard-open-defects">{data.openDefects}</div>
              <div className="dash-kpi-note">{data.openDefects === 0 ? 'none linked' : 'linked to failed cases'}</div>
            </div>
          </div>
        )}

        {!hidden.has('plans') && (
          <section data-tid="dashboard-plans" style={{ marginBottom: 16 }}>
            <h5 className="dash-h5">Test plans</h5>
            {data.plans.length === 0 ? (
              <div data-tid="dashboard-plans-empty" className="dash-muted">
                No active plans.{' '}
                {!isViewer && (
                  <button type="button" className="btn btn-secondary" data-tid="dashboard-create-plan" onClick={() => onNavigate('plans')}>Create your first plan</button>
                )}
              </div>
            ) : (
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill,minmax(260px,1fr))', gap: 12 }}>
                {data.plans.map((p) => {
                  const total = p.totalCases ?? 0
                  const pct = total > 0 ? Math.round((p.executedCases / total) * 100) : 0
                  return (
                    <div key={p.id} data-tid="dashboard-plan-card" data-plan={p.id} data-favorite={p.favorite} className="dash-repo-tests">
                      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                        <button type="button" data-tid="dashboard-plan-star" aria-pressed={p.favorite} aria-label={p.favorite ? `Unfavourite ${p.name}` : `Favourite ${p.name}`} onClick={() => void toggleFavorite(p)} style={{ background: 'none', border: 'none', cursor: 'pointer', fontSize: 16 }}>
                          {p.favorite ? '★' : '☆'}
                        </button>
                        <strong data-tid="dashboard-plan-name">{p.name}</strong>
                        <span className="tag" style={{ marginLeft: 'auto' }} data-tid="dashboard-plan-state">{p.stateLabel}</span>
                      </div>
                      <div role="progressbar" aria-valuemin={0} aria-valuemax={total} aria-valuenow={p.executedCases} style={{ display: 'flex', height: 8, background: '#e2e8f0', margin: '8px 0' }}>
                        <div style={{ width: `${pct}%`, background: '#16a34a' }} />
                      </div>
                      <div className="dash-muted" data-tid="dashboard-plan-progress">
                        {p.totalCases != null ? `${p.executedCases} / ${p.totalCases} cases executed` : 'not synced yet'}
                        {p.dueDate ? ` · due ${p.dueDate}` : ''}
                      </div>
                      <button type="button" className="btn btn-secondary" data-tid="dashboard-plan-action" disabled={isViewer && p.action === 'Start run'} title={isViewer && p.action === 'Start run' ? "Viewers can't start runs" : undefined} onClick={() => planAction(p)} style={{ marginTop: 8 }}>
                        {p.action}
                      </button>
                    </div>
                  )
                })}
              </div>
            )}
          </section>
        )}

        <div className="dash-content">
          {/* The design's pass-rate trend (left column); the Repositories panel is the right column. */}
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
                <div key={i} className="dash-chart-label">{t.run}</div>
              ))}
            </div>
          </div>
          <div className="dash-side" style={{ flex: 1 }}>
            {!hidden.has('repositories') && <RepositorySyncPanel workspaces={workspaces} currentWorkspaceId={currentWorkspaceId} onNavigate={onNavigate} details={data.repositories} />}
          </div>
        </div>

        <div className="dash-bottom">
          {!hidden.has('runs') && (
            <div className="dash-runs" data-tid="dashboard-runs" style={{ overflowX: 'auto', minWidth: 0 }}>
              <h5 className="dash-h5 dash-h5--flush">Runs</h5>
              {data.recentRuns.length === 0 ? (
                <div data-tid="dashboard-runs-empty" className="dash-muted">
                  No runs yet.{' '}
                  {data.plans.length > 0 && !isViewer && (
                    <button type="button" className="btn btn-secondary" data-tid="dashboard-start-run" onClick={() => onNavigate('plans')}>Start a run</button>
                  )}
                </div>
              ) : (
                <table className="table" data-tid="dashboard-runs-table">
                  <thead>
                    <tr><th>Run</th><th>Plan</th><th style={{ width: '30%' }}>Results</th><th>Pass rate</th><th>Source</th><th>Started</th></tr>
                  </thead>
                  <tbody>
                    {data.recentRuns.map((r) => (
                      <tr key={r.id} data-tid="dashboard-run-row" data-run={r.number} style={{ cursor: 'pointer' }} onClick={() => { requestOpenRun(r.id); onNavigate('reports') }}>
                        <td style={{ fontWeight: 600 }}>#{r.number}</td>
                        <td>{r.planName}</td>
                        <td><StatusBar tid="dashboard-run-bar" counts={{ passed: r.tally.passed, failed: r.tally.failed, blocked: r.tally.blocked, skipped: r.tally.skipped, untested: r.tally.untested }} /></td>
                        <td>
                          <span data-tid="dashboard-run-verdict" data-verdict={r.verdict} style={{ fontWeight: 700, color: r.verdict === 'PASSED' ? '#16a34a' : r.verdict === 'FAILED' ? 'var(--color-accent, #d32f2f)' : '#64748b' }}>
                            {r.passRate == null ? '—' : `${r.passRate}% · ${r.verdict}`}
                          </span>
                          {r.provisional ? <span className="dash-muted"> in progress</span> : null}
                        </td>
                        <td>{r.origin === 'CI' ? 'CI' : 'Manual'}</td>
                        <td className="dash-muted">{new Date(r.createdAt).toISOString().slice(0, 10)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          )}

          {!hidden.has('activity') && (
            <div className="dash-activity" data-tid="dashboard-activity">
              <h5 className="dash-h5 dash-h5--flush">Activity</h5>
              {activity.length === 0 && <div data-tid="dashboard-activity-empty" className="dash-muted">Nothing has happened yet.</div>}
              {groups.map((g) => {
                const items = activity.filter((a) => dateGroup(a.occurredAt) === g)
                if (items.length === 0) return null
                return (
                  <div key={g} data-tid="dashboard-activity-group" data-group={g}>
                    <div className="dash-eyebrow" style={{ marginTop: 8 }}>{g}</div>
                    {items.map((a) => (
                      <div key={a.id} className="dash-activity-item" data-tid="dashboard-activity-item">
                        <div>{describe(a)}</div>
                        <div className="dash-activity-when">{new Date(a.occurredAt).toLocaleString()}</div>
                      </div>
                    ))}
                  </div>
                )
              })}
              {nextCursor != null && (
                <button type="button" className="btn btn-secondary" data-tid="dashboard-activity-more" onClick={() => void showMore()}>Show more</button>
              )}
            </div>
          )}
        </div>

        <footer data-tid="dashboard-quicklinks" className="dash-actions" style={{ marginTop: 24, flexWrap: 'wrap' }}>
          <button type="button" className="btn btn-secondary" data-tid="dashboard-link-library" onClick={() => onNavigate('library')}>Library</button>
          <button type="button" className="btn btn-secondary" data-tid="dashboard-link-plans" onClick={() => onNavigate('plans')}>Test plans ({data.quickLinks.plans})</button>
          <button type="button" className="btn btn-secondary" data-tid="dashboard-link-runs" onClick={() => onNavigate('runs')}>Runs · unexecuted cases ({data.quickLinks.unexecutedCases})</button>
          <button type="button" className="btn btn-secondary" data-tid="dashboard-link-reports" onClick={() => onNavigate('reports')}>Reports</button>
          <button type="button" className="btn btn-secondary" data-tid="dashboard-link-settings" onClick={() => onNavigate('settings')}>Settings</button>
        </footer>
      </main>
    </div>
  )
}
