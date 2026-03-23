import { useState, useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import Board from './components/Board'
import ActivityFeed from './components/ActivityFeed'
import AgentsPanel from './components/AgentsPanel'
import StatsBar from './components/StatsBar'
import CreateTaskModal from './components/CreateTaskModal'
import { Task } from './types'
import { PlusCircle, Activity, Users, LayoutDashboard } from 'lucide-react'

type SidePanel = 'activity' | 'agents' | null

export default function App() {
  const queryClient = useQueryClient()
  const [sidePanel, setSidePanel] = useState<SidePanel>(null)
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [selectedTask, setSelectedTask] = useState<Task | null>(null)

  useEffect(() => {
    const es = new EventSource('/events')
    es.onmessage = (e) => {
      try {
        const { event } = JSON.parse(e.data)
        if (event === 'task_created' || event === 'task_updated') {
          queryClient.invalidateQueries({ queryKey: ['tasks'] })
          queryClient.invalidateQueries({ queryKey: ['stats'] })
          if (selectedTask) queryClient.invalidateQueries({ queryKey: ['task', selectedTask.id] })
        }
        if (event === 'activity_added' || event === 'task_created' || event === 'task_updated') {
          queryClient.invalidateQueries({ queryKey: ['activity'] })
        }
        if (event === 'agent_registered') {
          queryClient.invalidateQueries({ queryKey: ['agents'] })
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
          <span className="text-slate-500 text-sm">Task Tracker</span>
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
            {sidePanel === 'agents' && <AgentsPanel onTaskSelect={setSelectedTask} />}
          </aside>
        )}
      </div>

      {showCreateModal && <CreateTaskModal onClose={() => setShowCreateModal(false)} />}
    </div>
  )
}
