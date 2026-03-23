import { useQuery } from '@tanstack/react-query'
import { fetchAgents, fetchTasks } from '../api'
import { Agent, Task, AgentRole } from '../types'
import AgentBadge from './AgentBadge'
import { formatDistanceToNow } from 'date-fns'
import { Users } from 'lucide-react'

interface AgentsPanelProps {
  onTaskSelect: (task: Task) => void
}

function isActive(agent: Agent): boolean {
  if (!agent.last_seen) return false
  return Date.now() - new Date(agent.last_seen).getTime() < 5 * 60 * 1000
}

export default function AgentsPanel({ onTaskSelect }: AgentsPanelProps) {
  const { data: agents = [] } = useQuery({
    queryKey: ['agents'],
    queryFn: fetchAgents,
    refetchInterval: 15_000,
  })

  const { data: tasks = [] } = useQuery({
    queryKey: ['tasks'],
    queryFn: () => fetchTasks(),
  })

  const taskMap = Object.fromEntries(tasks.map(t => [t.id, t]))

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-2 px-4 py-3 border-b border-slate-800 shrink-0">
        <Users className="w-4 h-4 text-indigo-400" />
        <span className="text-sm font-semibold text-white">Agents</span>
        <span className="ml-auto text-xs text-slate-500">{agents.length} registered</span>
      </div>
      <div className="flex-1 overflow-y-auto p-3 space-y-2">
        {agents.length === 0 && (
          <p className="text-center text-slate-600 text-sm mt-8">No agents registered</p>
        )}
        {(agents as Agent[]).map(agent => {
          const active = isActive(agent)
          const currentTask = agent.current_task_id ? taskMap[agent.current_task_id] : null
          return (
            <div
              key={agent.agent_id}
              className={`rounded-lg p-2.5 border transition-colors ${active ? 'bg-slate-800 border-slate-700' : 'bg-slate-900 border-slate-800 opacity-60'}`}
            >
              <div className="flex items-center justify-between mb-1.5">
                <AgentBadge role={agent.role as AgentRole} size="md" />
                <span className={`text-xs font-medium ${active ? 'text-emerald-400' : 'text-slate-600'}`}>
                  {active ? '● active' : '○ inactive'}
                </span>
              </div>
              <p className="text-xs font-mono text-slate-300 mb-1">{agent.agent_id}</p>
              {agent.last_seen && (
                <p className="text-xs text-slate-600">
                  Last seen {formatDistanceToNow(new Date(agent.last_seen), { addSuffix: true })}
                </p>
              )}
              {currentTask && (
                <button
                  onClick={() => onTaskSelect(currentTask)}
                  className="mt-1.5 text-xs text-indigo-400 hover:text-indigo-300 truncate w-full text-left"
                >
                  → {currentTask.title}
                </button>
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}
