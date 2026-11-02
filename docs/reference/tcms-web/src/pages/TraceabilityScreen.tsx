import { useEffect, useState } from 'react'
import { User, Workspace } from '../types'
import Header from '../components/Header'
import { testRunApi, NO_DATA_MESSAGE } from '../utils/api'
import { runsApi } from '../utils/runsApi'
import { traceApi } from '../utils/traceApi'
import { useJiraProject } from '../utils/useJiraProject'
import { useProduct } from '../utils/product'
import { Run } from '../types/runs'
import { SelectedCase } from '../utils/selectedCase'
import PlanTraceability from './traceability/PlanTraceability'
import TraceabilityDocumentButton from '../components/TraceabilityDocumentButton'
import RunTestsButton from '../components/RunTestsButton'
import { saveBlob } from '../utils/documentsApi'
import { canAdminister, canManage, currentRole } from '../utils/roles'
import '../styles/traceability.css'

export interface TraceabilityScreenProps {
  /** Opens a case in the case detail (a repository and a file, D-26). */
  onOpenCase?: (selected: SelectedCase | null) => void
  user: User
  workspaces?: Workspace[]
  currentWorkspaceId?: number | null
  onSelectWorkspace?: (workspace: Workspace) => void
  onNavigate: (screen: string) => void
  onSignOut: () => void
}

type ResultStatus = 'pass' | 'fail' | 'blocked'

interface ResultStyle {
  bg: string
  bd: string
  label: string
}

const RESULT_STYLES: Record<ResultStatus, ResultStyle> = {
  pass: { bg: '#dcfce7', bd: '#16a34a', label: 'Pass' },
  fail: { bg: '#fee2e2', bd: '#dc2626', label: 'Fail' },
  blocked: { bg: '#fef3c7', bd: '#d97706', label: 'Blocked' },
}

interface TraceRequirement {
  id: string
  key: string
  summary: string
  cases: string[]
  results: ResultStatus[]
  defect: string | null
  defectState: string | null
  coverage: string
  coverageTone: 'ok' | 'warn' | 'none'
}


const coverageTagClass = (tone: TraceRequirement['coverageTone']) => {
  switch (tone) {
    case 'ok':
      return 'bg-green-100 text-green-800'
    case 'warn':
      return 'bg-yellow-100 text-yellow-800'
    default:
      return 'bg-neutral-100 text-neutral-700'
  }
}

/** One CSV cell: quoted when it holds a comma, quote or line break. */
const csvCell = (value: string) => (/[",\n\r]/.test(value) ? `"${value.replace(/"/g, '""')}"` : value)

/** The V&V matrix as CSV, built in the browser from the rows on screen (D-552). */
function vvTraceabilityCsv(rows: TraceRequirement[]): string {
  const lines = [['Requirement', 'Summary', 'Test cases', 'Latest results', 'Defect', 'Defect state', 'Coverage']]
  for (const r of rows) {
    lines.push([r.key, r.summary, r.cases.join(' '), r.results.map((x) => RESULT_STYLES[x]?.label ?? x).join(' '), r.defect ?? '', r.defectState ?? '', r.coverage])
  }
  return lines.map((l) => l.map((c) => csvCell(String(c ?? ''))).join(',')).join('\r\n') + '\r\n'
}

export function VvTraceability({ user, workspaces = [], currentWorkspaceId, onSelectWorkspace, onNavigate, onSignOut }: TraceabilityScreenProps) {
  const { productId, product } = useProduct()
  const [requirements, setRequirements] = useState<TraceRequirement[]>([])
  const [emptyMessage, setEmptyMessage] = useState<string | null>(null)

  useEffect(() => {
    const fetchRequirements = async () => {
      try {
        const vvResults = await testRunApi.latestResults()
        if (!vvResults || !vvResults.testCases || vvResults.testCases.length === 0) {
          setRequirements([])
          setEmptyMessage(NO_DATA_MESSAGE)
          return
        }

        const data = (await testRunApi.traceability(vvResults.runId)) as TraceRequirement[]
        if (!data || data.length === 0) {
          setRequirements([])
          setEmptyMessage(NO_DATA_MESSAGE)
          return
        }

        setRequirements(data)
        setEmptyMessage(null)
      } catch {
        setRequirements([])
        setEmptyMessage(NO_DATA_MESSAGE)
      }
    }
    fetchRequirements()
  }, [])

  // The context line names only what the workspace holds: the product and, with Jira, its project.
  const jiraProject = useJiraProject(currentWorkspaceId)
  const context = [product?.name ?? 'All products', jiraProject ? `Jira project ${jiraProject}` : null].filter(Boolean).join(' · ')

  const day = () => new Date().toISOString().slice(0, 10)
  const exportCsv = () => saveBlob(new Blob([vvTraceabilityCsv(requirements)], { type: 'text/csv;charset=utf-8' }), `traceability_vv_${day()}.csv`)
  const exportPdf = () => {
    const previous = document.title
    document.title = `traceability_vv_${day()}`
    const restore = () => {
      document.title = previous
      window.removeEventListener('afterprint', restore)
    }
    window.addEventListener('afterprint', restore)
    window.print()
  }

  const covered = requirements.filter((r) => r.coverageTone === 'ok').length
  const total = requirements.length
  const withDefect = requirements.filter((r) => r.defect).length

  return (
    <div className="min-h-screen bg-neutral-50" data-tid="traceability-screen" data-screen-label="Traceability">
      <Header user={user} workspaces={workspaces} currentWorkspaceId={currentWorkspaceId} onSelectWorkspace={onSelectWorkspace} currentPage="traceability" onNavigate={onNavigate} onSignOut={onSignOut} />

      <main className="max-w-7xl mx-auto px-6 py-8">
        <div className="space-y-6">
          <div
            data-tid="traceability-header"
            className="flex items-end justify-between gap-4 flex-wrap pb-4 border-b-2 border-neutral-200"
          >
            <div>
              <div
                data-tid="traceability-context"
                className="text-xs uppercase tracking-wider text-neutral-500"
              >
                {context}
              </div>
              <h1 className="mt-1 text-3xl font-bold text-neutral-900">Traceability</h1>
            </div>
            <div className="flex items-end gap-6 flex-wrap">
            <div data-tid="coverage-summary" className="flex gap-6 text-sm">
              <div>
                <strong className="text-2xl font-semibold text-neutral-900">
                  {covered}/{total}
                </strong>{' '}
                requirements covered
              </div>
              <div>
                <strong className="text-2xl font-semibold" style={{ color: 'var(--color-accent)' }}>
                  {withDefect}
                </strong>{' '}
                with open defect
              </div>
            </div>
              <span className="trace-noprint flex items-end gap-2">
                <button type="button" data-tid="traceability-export-pdf" className="btn btn-primary" onClick={exportPdf} title="Prints the matrix: choose Save as PDF">
                  Export PDF
                </button>
                <button type="button" data-tid="traceability-vv-export" className="btn btn-primary" onClick={exportCsv} disabled={requirements.length === 0}>
                  Export CSV
                </button>
                <TraceabilityDocumentButton repositoryId={productId} canGenerate={canManage(currentRole(workspaces, currentWorkspaceId))} canSetUp={canAdminister(currentRole(workspaces, currentWorkspaceId))} onNavigate={onNavigate} />
                <RunTestsButton repositoryId={productId} role={currentRole(workspaces, currentWorkspaceId)} place="traceability" primary onNavigate={onNavigate} />
              </span>
            </div>
          </div>

          <div
            data-tid="traceability-legend"
            className="flex items-center gap-4 text-xs text-neutral-600"
          >
            <span className="font-medium text-neutral-700">Latest results:</span>
            {(Object.keys(RESULT_STYLES) as ResultStatus[]).map((status) => (
              <span key={status} className="flex items-center gap-1.5">
                <span
                  className="inline-block"
                  style={{
                    width: 14,
                    height: 14,
                    background: RESULT_STYLES[status].bg,
                    border: `1px solid ${RESULT_STYLES[status].bd}`,
                  }}
                />
                {RESULT_STYLES[status].label}
              </span>
            ))}
          </div>

          {requirements.length === 0 ? (
            <div
              data-tid="traceability-no-data"
              className="bg-white rounded-lg border border-neutral-200 px-6 py-12 text-center text-sm text-neutral-500"
            >
              {emptyMessage ?? NO_DATA_MESSAGE}
            </div>
          ) : (
          <div data-tid="requirements-matrix" className="bg-white rounded-lg border border-neutral-200 overflow-hidden">
            <table className="w-full">
              <thead className="bg-neutral-50 border-b border-neutral-200">
                <tr>
                  <th className="px-6 py-3 text-left text-sm font-semibold text-neutral-900">Requirement</th>
                  <th className="px-6 py-3 text-left text-sm font-semibold text-neutral-900">Test cases</th>
                  <th className="px-6 py-3 text-left text-sm font-semibold text-neutral-900">Latest results</th>
                  <th className="px-6 py-3 text-left text-sm font-semibold text-neutral-900">Defects</th>
                  <th className="px-6 py-3 text-left text-sm font-semibold text-neutral-900">Coverage</th>
                </tr>
              </thead>
              <tbody>
                {requirements.map((req) => (
                  <tr
                    key={req.id}
                    data-tid={`traceability-row-${req.id}`}
                    className="border-b border-neutral-200 hover:bg-neutral-50 align-top"
                  >
                    <td className="px-6 py-3">
                      <a
                        href="#"
                        className="font-mono text-xs font-semibold"
                        style={{ color: 'var(--color-accent)' }}
                        onClick={(e) => e.preventDefault()}
                      >
                        {req.key}
                      </a>
                      <div className="text-sm text-neutral-900">{req.summary}</div>
                    </td>
                    <td className="px-6 py-3">
                      <div data-tid={`traceability-cases-${req.id}`} className="flex gap-1 flex-wrap">
                        {req.cases.map((c) => (
                          <span
                            key={c}
                            className="px-2 py-0.5 rounded text-xs font-mono bg-neutral-100 text-neutral-700"
                          >
                            {c}
                          </span>
                        ))}
                        {req.cases.length === 0 && (
                          <span className="text-sm text-neutral-500">No tests linked</span>
                        )}
                      </div>
                    </td>
                    <td className="px-6 py-3">
                      <div data-tid={`traceability-results-${req.id}`} className="flex gap-1">
                        {req.results.map((r, i) => (
                          <span
                            key={i}
                            title={RESULT_STYLES[r].label}
                            style={{
                              width: 18,
                              height: 18,
                              display: 'inline-block',
                              background: RESULT_STYLES[r].bg,
                              border: `1px solid ${RESULT_STYLES[r].bd}`,
                            }}
                          />
                        ))}
                        {req.results.length === 0 && (
                          <span className="text-sm text-neutral-500">—</span>
                        )}
                      </div>
                    </td>
                    <td className="px-6 py-3">
                      <div data-tid={`traceability-defects-${req.id}`}>
                        {req.defect ? (
                          <>
                            <a
                              href="#"
                              className="text-sm font-medium"
                              style={{ color: 'var(--color-accent)' }}
                              onClick={(e) => e.preventDefault()}
                            >
                              {req.defect}
                            </a>
                            <div className="text-xs text-neutral-500">{req.defectState}</div>
                          </>
                        ) : (
                          <span className="text-sm text-neutral-400">None</span>
                        )}
                      </div>
                    </td>
                    <td className="px-6 py-3">
                      <span
                        data-tid={`traceability-coverage-${req.id}`}
                        className={`px-2 py-1 rounded-full text-xs font-medium ${coverageTagClass(req.coverageTone)}`}
                      >
                        {req.coverage}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          )}
        </div>
      </main>
    </div>
  )
}

/**
 * Traceability (ORCH-018). A workspace with plan runs gets the requirements of its test repository
 * covered by those runs (D-116); one with none keeps the view of the loaded V&V run.
 */
export default function TraceabilityScreen(props: TraceabilityScreenProps) {
  const { user, workspaces = [], currentWorkspaceId, onSelectWorkspace, onNavigate, onSignOut } = props
  const [runs, setRuns] = useState<Run[] | null>(null)
  // Without a run: does the product's synced cases cite requirements? Then they show, all Not run (D-130).
  const [cited, setCited] = useState<boolean | null>(null)
  const { productId } = useProduct()

  useEffect(() => {
    let alive = true
    setRuns(null)
    setCited(null)
    runsApi
      .list({ repositoryId: productId })
      .then(async (list) => {
        if (list.length === 0) {
          const t = await traceApi.get({ repositoryId: productId ?? undefined }).catch(() => null)
          if (alive) setCited((t?.requirements.length ?? 0) > 0)
        }
        if (alive) setRuns(list)
      })
      .catch(() => alive && setRuns([]))
    return () => {
      alive = false
    }
  }, [currentWorkspaceId, productId])

  if (runs == null) {
    return (
      <div className="min-h-screen bg-neutral-50" data-tid="traceability-screen" data-screen-label="Traceability">
        <Header user={user} workspaces={workspaces} currentWorkspaceId={currentWorkspaceId} onSelectWorkspace={onSelectWorkspace} currentPage="traceability" onNavigate={onNavigate} onSignOut={onSignOut} />
        <main className="max-w-7xl mx-auto px-6 py-8">
          <p data-tid="traceability-loading">Loading traceability...</p>
        </main>
      </div>
    )
  }
  if (runs.length === 0) {
    return cited ? <PlanTraceability {...props} runs={[]} repositoryId={productId} /> : <VvTraceability {...props} />
  }
  return <PlanTraceability {...props} runs={runs} />
}
