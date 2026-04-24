import { useState, useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useQuery } from '@tanstack/react-query'
import Board from './components/Board'
import ActivityFeed from './components/ActivityFeed'
import AgentsPanel from './components/AgentsPanel'
import StatsBar from './components/StatsBar'
import CreateTaskModal from './components/CreateTaskModal'
import AgentLogModal from './components/AgentLogModal'
import { fetchInfo } from './api'
import { Task, AgentLogLine } from './types'
import { PlusCircle, Activity, Users, LayoutDashboard, FolderGit2 } from 'lucide-react'

type SidePanel = 'activity' | 'agents' | null

const MAX_LOG_LINES = 1000

export default function App() {
  const queryClient = useQueryClient()
  const { data: info } = useQuery({ queryKey: ['info'], queryFn: fetchInfo, staleTime: Infinity })
  const [sidePanel, setSidePanel] = useState<SidePanel>(null)
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [selectedTask, setSelectedTask] = useState<Task | null>(null)
  const [agentLogs, setAgentLogs] = useState<Record<string, AgentLogLine[]>>({})
  const [logAgentId, setLogAgentId] = useState<string | null>(null)

  useEffect(() => {
    const es = new EventSource('/events')
    es.onmessage = (e) => {
      try {
        const { event, data } = JSON.parse(e.data)
        if (event === 'task_created' || event === 'task_updated') {
          queryClient.invalidateQueries({ queryKey: ['tasks'] })
          queryClient.invalidateQueries({ queryKey: ['stats'] })
          if (selectedTask) queryClient.invalidateQueries({ queryKey: ['task', selectedTask.id] })
        }
        if (event === 'activity_added' || event === 'task_created' || event === 'task_updated') {
          queryClient.invalidateQueries({ queryKey: ['activity'] })
        }
        if (event === 'agent_registered' || event === 'agent_updated') {
          queryClient.invalidateQueries({ queryKey: ['agents'] })
        }
        if (event === 'agent_log' && data?.agent_id) {
          setAgentLogs(prev => {
            const existing = prev[data.agent_id] ?? []
            const next = [...existing, data as AgentLogLine]
            return {
              ...prev,
              [data.agent_id]: next.length > MAX_LOG_LINES
                ? next.slice(next.length - MAX_LOG_LINES)
                : next,
            }
          })
        }
      } catch { /* ignore parse errors */ }
    }
    return () => es.close()
  }, [queryClient, selectedTask])

  return (
    <div className="flex flex-col h-screen overflow-hidden bg-slate-950">
      {/* Header */}
      <header className="flex items-center justify-between px-6 py-3 border-b border-slate-800 bg-slate-900 shrink-0">
        <div className="flex items-center gap-3">
          <LayoutDashboard className="w-5 h-5 text-indigo-400" />
          <span className="font-semibold text-white tracking-tight">Mandatum</span>
          {info?.repo_path ? (
            <span className="flex items-center gap-1.5 text-xs text-slate-400 bg-slate-800 px-2 py-0.5 rounded font-mono">
              <FolderGit2 className="w-3.5 h-3.5 text-indigo-400 shrink-0" />
              {info.repo_path}
            </span>
          ) : (
            <span className="text-slate-500 text-sm">Task Tracker</span>
          )}
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setSidePanel(sidePanel === 'activity' ? null : 'activity')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm transition-colors ${sidePanel === 'activity' ? 'bg-indigo-600 text-white' : 'text-slate-400 hover:text-white hover:bg-slate-800'}`}
          >
            <Activity className="w-4 h-4" />Activity
          </button>
          <button
            onClick={() => setSidePanel(sidePanel === 'agents' ? null : 'agents')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm transition-colors ${sidePanel === 'agents' ? 'bg-indigo-600 text-white' : 'text-slate-400 hover:text-white hover:bg-slate-800'}`}
          >
            <Users className="w-4 h-4" />Agents
          </button>
          <button
            onClick={() => setShowCreateModal(true)}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm bg-indigo-600 hover:bg-indigo-500 text-white transition-colors"
          >
            <PlusCircle className="w-4 h-4" />New Task
          </button>
        </div>
      </header>

      <StatsBar />

      <div className="flex flex-1 overflow-hidden">
        <main className="flex-1 overflow-hidden">
          <Board onTaskSelect={setSelectedTask} selectedTask={selectedTask} />
        </main>
        {sidePanel && (
          <aside className="w-80 shrink-0 border-l border-slate-800 bg-slate-900 overflow-hidden flex flex-col">
            {sidePanel === 'activity' && <ActivityFeed />}
            {sidePanel === 'agents' && (
              <AgentsPanel
                onTaskSelect={setSelectedTask}
                onViewLog={(id) => { setLogAgentId(id); setSidePanel('agents') }}
              />
            )}
          </aside>
        )}
      </div>

      {showCreateModal && <CreateTaskModal onClose={() => setShowCreateModal(false)} />}
      {logAgentId && (
        <AgentLogModal
          agentId={logAgentId}
          liveLogs={agentLogs[logAgentId] ?? []}
          onClose={() => setLogAgentId(null)}
        />
      )}
    </div>
  )
}
