import { Task, TaskPriority, AgentRole, Agent } from '../types'
import AgentBadge from './AgentBadge'
import { formatDistanceToNow } from 'date-fns'
import { GitBranch, GitCommit, AlertTriangle, Zap, Link } from 'lucide-react'

export const PRIORITY_CONFIG: Record<TaskPriority, { label: string; bg: string; text: string; dot: string }> = {
  critical: { label: 'Critical', bg: 'bg-red-900/60',    text: 'text-red-300',    dot: 'bg-red-400'    },
  high:     { label: 'High',     bg: 'bg-orange-900/60', text: 'text-orange-300', dot: 'bg-orange-400' },
  medium:   { label: 'Medium',   bg: 'bg-yellow-900/60', text: 'text-yellow-300', dot: 'bg-yellow-400' },
  low:      { label: 'Low',      bg: 'bg-slate-800',     text: 'text-slate-400',  dot: 'bg-slate-500'  },
}

const STALE_MS = 10 * 60 * 1000 // 10 minutes

function agentIsStale(agent: Agent | undefined): boolean {
  if (!agent?.last_seen) return false
  return Date.now() - new Date(agent.last_seen).getTime() > STALE_MS
}

interface TaskCardProps {
  task: Task
  agent?: Agent
  allTasks?: Task[]
  onClick: () => void
  onDragStart: (e: React.DragEvent) => void
}

const ACTIVE_AGENT_MS = 5 * 60 * 1000 // 5 minutes

function agentIsActive(agent: Agent | undefined): boolean {
  if (!agent?.last_seen) return false
  return Date.now() - new Date(agent.last_seen).getTime() < ACTIVE_AGENT_MS
}

export default function TaskCard({ task, agent, allTasks = [], onClick, onDragStart }: TaskCardProps) {
  const p = PRIORITY_CONFIG[task.priority]
  const stale = agentIsStale(agent)
  const claimed = !!task.assigned_agent_id
  const active = agentIsActive(agent)
  const blockedByDeps = task.dependencies.length > 0 &&
    task.dependencies.some(depId => {
      const dep = allTasks.find(t => t.id === depId)
      return !dep || dep.status !== 'done'
    })

  return (
    <div
      draggable
      onDragStart={onDragStart}
      onClick={onClick}
      className={`group rounded-lg p-3 cursor-pointer transition-all duration-150 select-none border ${
        claimed
          ? 'bg-blue-950/40 border-blue-700/60 hover:border-blue-500 ring-1 ring-blue-800/40'
          : 'bg-slate-800 border-slate-700 hover:border-slate-500'
      }`}
    >
      {/* Agent claim banner */}
      {claimed && (
        <div className={`flex items-center gap-1.5 -mx-3 -mt-3 mb-3 px-3 py-1.5 rounded-t-lg border-b ${
          stale
            ? 'bg-amber-900/40 border-amber-700/40'
            : 'bg-blue-900/50 border-blue-700/40'
        }`}>
          {stale ? (
            <AlertTriangle className="w-3 h-3 shrink-0 text-amber-400" />
          ) : (
            <Zap className={`w-3 h-3 shrink-0 text-blue-400 ${active ? 'animate-pulse' : ''}`} />
          )}
          <span className={`text-xs font-semibold ${stale ? 'text-amber-300' : 'text-blue-300'}`}>
            {stale ? 'Agent stale' : 'In progress'}
          </span>
          <span className="ml-auto text-xs font-mono truncate text-slate-400 max-w-[100px]">
            {task.assigned_agent_id}
          </span>
        </div>
      )}

      <div className="flex items-center justify-between mb-2 gap-1">
        <span className={`inline-flex items-center gap-1 text-xs rounded px-1.5 py-0.5 font-medium ${p.bg} ${p.text}`}>
          <span className={`w-1.5 h-1.5 rounded-full ${p.dot}`} />
          {p.label}
        </span>
        {task.assigned_role && <AgentBadge role={task.assigned_role as AgentRole} />}
      </div>

      <p className="text-sm font-medium text-slate-100 leading-snug mb-1 line-clamp-2 group-hover:text-white">
        {task.title}
      </p>
      <p className="text-xs font-mono text-slate-600 mb-2 truncate">{task.id.slice(0, 8)}</p>

      {task.tags.length > 0 && (
        <div className="flex flex-wrap gap-1 mb-2">
          {task.tags.slice(0, 3).map(tag => (
            <span key={tag} className="text-xs bg-slate-700 text-slate-400 rounded px-1.5 py-0.5">{tag}</span>
          ))}
          {task.tags.length > 3 && (
            <span className="text-xs text-slate-500">+{task.tags.length - 3}</span>
          )}
        </div>
      )}

      {/* Dependencies */}
      {task.dependencies.length > 0 && (
        <div className={`flex items-center gap-1 text-xs mt-1 ${blockedByDeps ? 'text-amber-400' : 'text-emerald-500'}`}>
          <Link className="w-3 h-3 shrink-0" />
          <span>{task.dependencies.length} dep{task.dependencies.length !== 1 ? 's' : ''}</span>
          {blockedByDeps && <span className="ml-0.5 text-amber-500">· waiting</span>}
        </div>
      )}

      {/* Git info */}
      {task.branch_name && (
        <div className="flex items-center gap-1 text-xs text-slate-500 font-mono truncate mt-1">
          <GitBranch className="w-3 h-3 shrink-0 text-slate-600" />
          <span className="truncate">{task.branch_name}</span>
          {task.commit_count > 0 && (
            <span className="flex items-center gap-0.5 shrink-0 ml-auto text-slate-600">
              <GitCommit className="w-3 h-3" />{task.commit_count}
            </span>
          )}
        </div>
      )}

      {/* Stale agent last-seen */}
      {stale && agent?.last_seen && (
        <div className="flex items-center gap-1 mt-1.5 text-xs text-amber-500/80">
          <span>Last seen {formatDistanceToNow(new Date(agent.last_seen), { addSuffix: true })}</span>
        </div>
      )}
    </div>
  )
}
