import { useQuery } from '@tanstack/react-query'
import { fetchActivity } from '../api'
import { ActivityEntry, AgentRole, GIT_ACTIONS } from '../types'
import AgentBadge from './AgentBadge'
import { formatDistanceToNow } from 'date-fns'
import { Activity, GitCommit } from 'lucide-react'

export default function ActivityFeed() {
  const { data: entries = [] } = useQuery({
    queryKey: ['activity'],
    queryFn: fetchActivity,
    refetchInterval: 10_000,
  })

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-2 px-4 py-3 border-b border-slate-800 shrink-0">
        <Activity className="w-4 h-4 text-indigo-400" />
        <span className="text-sm font-semibold text-white">Activity Feed</span>
      </div>
      <div className="flex-1 overflow-y-auto p-3 space-y-2">
        {entries.length === 0 && (
          <p className="text-center text-slate-600 text-sm mt-8">No activity yet</p>
        )}
        {(entries as ActivityEntry[]).map(entry => {
          const isGit = GIT_ACTIONS.has(entry.action)
          return (
            <div key={entry.id} className={`rounded-lg p-2.5 space-y-1 ${isGit ? 'bg-slate-800/60 border border-slate-700/50' : 'bg-slate-800'}`}>
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-1.5 min-w-0">
                  {isGit && <GitCommit className="w-3 h-3 text-emerald-600 shrink-0" />}
                  {entry.agent_role && <AgentBadge role={entry.agent_role as AgentRole} />}
                  <span className="text-xs text-slate-300 font-mono truncate">
                    {entry.agent_id ?? 'System'}
                  </span>
                </div>
                <span className="text-xs text-slate-600 shrink-0">
                  {formatDistanceToNow(new Date(entry.timestamp), { addSuffix: true })}
                </span>
              </div>
              <p className="text-xs text-slate-400">
                <span className={`font-medium ${isGit ? 'text-emerald-400' : 'text-slate-300'}`}>
                  {entry.action.replace(/_/g, ' ')}
                </span>
                {entry.detail && ` — ${entry.detail}`}
              </p>
            </div>
          )
        })}
      </div>
    </div>
  )
}
