import { useEffect, useState } from 'react'
import { Run, RunResults } from '../../types/runs'
import { resultsApi } from '../../utils/resultsApi'
import { FOLDER_RUN_STATUS_TEXT, isServerFolder } from '../../utils/folderSource'
import { useRepositorySync } from '../../utils/repositorySync'

interface Props {
  run: Run
  /** The caller may write to this run (a member on an open run). */
  canWrite: boolean
}

const short = (sha: string | null) => (sha ? sha.slice(0, 7) : '')

/**
 * Run header line: where the run record lives in git and the "Commit results" action (ORCH-029.03,
 * DECISIONS.md D-107, D-108). NOT_CONFIGURED is left only on runs from before D-304 (new writes are refused without a write configuration); COMMIT_FAILED (a CI-created run
 * whose commit failed) shows the error and offers a retry.
 */
export default function RunResultsStatus({ run, canWrite }: Props) {
  // The latest results after a commit from here, until the next poll brings the run in again.
  const [override, setOverride] = useState<{ runId: number; results: RunResults } | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const { repos } = useRepositorySync()
  const folderRun = isServerFolder(repos.find((r) => r.id === run.repository?.id)?.sourceType)

  useEffect(() => {
    setOverride(null)
    setError(null)
  }, [run.id])

  const results = override?.runId === run.id ? override.results : run.results
  if (!results) return null

  const commit = async () => {
    setBusy(true)
    setError(null)
    try {
      const updated = await resultsApi.commitResults(run.id)
      if (updated.results) setOverride({ runId: run.id, results: updated.results })
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Commit failed')
    } finally {
      setBusy(false)
    }
  }

  return (
    <div data-tid="run-results" data-status={results.status} className="text-[12px] text-neutral-500 mt-1 flex items-center gap-2 flex-wrap">
      {results.status === 'COMMITTED' && (
        <span data-tid="run-results-status">
          Results committed to <code>{results.branch}</code>
          {results.commitSha ? ` @ ${short(results.commitSha)}` : ''}
          {results.mergeRequestUrl && (
            <>
              {' · '}
              <a data-tid="run-results-mr" href={results.mergeRequestUrl} target="_blank" rel="noreferrer" className="underline">
                merge request #{results.mergeRequestNumber}
              </a>
            </>
          )}
          <span data-tid="run-results-plain">
            {' · '}Every result you record is saved in git for you.{results.mergeRequestUrl ? ' Your team reviews the run in the merge request.' : ''}
          </span>
        </span>
      )}
      {results.status === 'NOT_CONFIGURED' && (
        <span data-tid="run-results-status">
          {folderRun
            ? FOLDER_RUN_STATUS_TEXT
            : 'Not in git: this run’s test repository has no write configuration, so new results are refused. An owner or admin sets it in Settings › Repositories › Write results to git.'}
        </span>
      )}
      {results.status === 'COMMIT_FAILED' && (
        <span data-tid="run-results-status" className="text-red-700">
          Committing the run record failed{results.error ? `: ${results.error}` : ''}
        </span>
      )}
      {canWrite && run.status === 'OPEN' && (
        <button
          type="button"
          data-tid="run-commit-results"
          className="btn btn-secondary"
          style={{ padding: '2px 8px', fontSize: 12 }}
          disabled={busy}
          title="Saves the run in git again right now, with everything recorded so far"
          onClick={commit}
        >
          {busy ? 'Committing…' : results.status === 'COMMIT_FAILED' ? 'Retry commit' : 'Commit results'}
        </button>
      )}
      {error && (
        <span data-tid="run-results-error" role="alert" className="w-full text-red-700">
          {error}
        </span>
      )}
    </div>
  )
}
