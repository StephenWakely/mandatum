import { Task, TaskPriority, AgentRole } from '../types'
import AgentBadge from './AgentBadge'
import { GitBranch, GitCommit } from 'lucide-react'

export const PRIORITY_CONFIG: Record<TaskPriority, { label: string; bg: string; text: string; dot: string }> = {
  critical: { label: 'Critical', bg: 'bg-red-900/60',    text: 'text-red-300',    dot: 'bg-red-400'    },
  high:     { label: 'High',     bg: 'bg-orange-900/60', text: 'text-orange-300', dot: 'bg-orange-400' },
  medium:   { label: 'Medium',   bg: 'bg-yellow-900/60', text: 'text-yellow-300', dot: 'bg-yellow-400' },
  low:      { label: 'Low',      bg: 'bg-slate-800',     text: 'text-slate-400',  dot: 'bg-slate-500'  },
}

interface TaskCardProps {
  task: Task
  onClick: () => void
  onDragStart: (e: React.DragEvent) => void
}

export default function TaskCard({ task, onClick, onDragStart }: TaskCardProps) {
  const p = PRIORITY_CONFIG[task.priority]

  return (
    <div
      draggable
      onDragStart={onDragStart}
      onClick={onClick}
      className="group bg-slate-800 border border-slate-700 rounded-lg p-3 cursor-pointer hover:border-slate-500 transition-all duration-150 select-none"
    >
      <div className="flex items-center justify-between mb-2 gap-1">
        <span className={`inline-flex items-center gap-1 text-xs rounded px-1.5 py-0.5 font-medium ${p.bg} ${p.text}`}>
          <span className={`w-1.5 h-1.5 rounded-full ${p.dot}`} />
          {p.label}
        </span>
        {task.assigned_role && <AgentBadge role={task.assigned_role as AgentRole} />}
      </div>

      <p className="text-sm font-medium text-slate-100 leading-snug mb-2 line-clamp-2 group-hover:text-white">
        {task.title}
      </p>

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

      {!task.branch_name && task.assigned_agent_id && (
        <p className="text-xs text-slate-500 font-mono truncate">{task.assigned_agent_id}</p>
      )}
    </div>
  )
}
