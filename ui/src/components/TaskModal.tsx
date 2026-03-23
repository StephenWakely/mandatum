import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { fetchTask, updateTask, deleteTask } from '../api'
import { Task, TaskStatus, TaskPriority, AgentRole } from '../types'
import AgentBadge from './AgentBadge'
import { PRIORITY_CONFIG } from './TaskCard'
import { formatDistanceToNow } from 'date-fns'
import { X, Copy, Pencil, Trash2, ChevronDown } from 'lucide-react'

const STATUSES: TaskStatus[] = ['backlog', 'in_progress', 'in_review', 'testing', 'docs_needed', 'done', 'blocked']
const PRIORITIES: TaskPriority[] = ['low', 'medium', 'high', 'critical']
const ROLES: AgentRole[] = ['coder', 'reviewer', 'tester', 'docs_writer']

interface TaskModalProps {
  task: Task
  onClose: () => void
}

export default function TaskModal({ task: initialTask, onClose }: TaskModalProps) {
  const queryClient = useQueryClient()
  const [editing, setEditing] = useState(false)
  const [showStatusMenu, setShowStatusMenu] = useState(false)
  const [editForm, setEditForm] = useState({
    title: initialTask.title,
    description: initialTask.description ?? '',
    priority: initialTask.priority,
    assigned_role: initialTask.assigned_role ?? '' as AgentRole | '',
    tags: initialTask.tags.join(', '),
  })

  const { data: task = initialTask } = useQuery({
    queryKey: ['task', initialTask.id],
    queryFn: () => fetchTask(initialTask.id),
  })

  const updateMutation = useMutation({
    mutationFn: (data: Parameters<typeof updateTask>[1]) => updateTask(task.id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tasks'] })
      queryClient.invalidateQueries({ queryKey: ['task', task.id] })
      queryClient.invalidateQueries({ queryKey: ['stats'] })
      setEditing(false)
    },
  })

  const deleteMutation = useMutation({
    mutationFn: () => deleteTask(task.id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tasks'] })
      queryClient.invalidateQueries({ queryKey: ['stats'] })
      onClose()
    },
  })

  const handleSave = () => {
    updateMutation.mutate({
      title: editForm.title,
      description: editForm.description || undefined,
      priority: editForm.priority as TaskPriority,
      assigned_role: (editForm.assigned_role as AgentRole) || undefined,
      tags: editForm.tags ? editForm.tags.split(',').map(t => t.trim()).filter(Boolean) : [],
    })
  }

  const handleStatusChange = (status: TaskStatus) => {
    updateMutation.mutate({ status })
    setShowStatusMenu(false)
  }

  const p = PRIORITY_CONFIG[task.priority]

  return (
    <div
      className="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-4"
      onClick={e => e.target === e.currentTarget && onClose()}
    >
      <div className="bg-slate-900 border border-slate-700 rounded-xl w-full max-w-2xl max-h-[85vh] flex flex-col shadow-2xl">
        {/* Header */}
        <div className="flex items-start justify-between p-5 border-b border-slate-800">
          {editing ? (
            <input
              value={editForm.title}
              onChange={e => setEditForm(f => ({ ...f, title: e.target.value }))}
              className="flex-1 bg-slate-800 border border-slate-600 rounded-lg px-3 py-1.5 text-white text-lg font-semibold mr-2 outline-none focus:border-indigo-500"
            />
          ) : (
            <h2 className="text-lg font-semibold text-white flex-1 mr-3 leading-snug">{task.title}</h2>
          )}
          <div className="flex items-center gap-1.5 shrink-0">
            {!editing && (
              <button onClick={() => setEditing(true)} className="p-1.5 rounded-md text-slate-400 hover:text-white hover:bg-slate-800 transition-colors">
                <Pencil className="w-4 h-4" />
              </button>
            )}
            <button
              onClick={() => { if (confirm('Delete this task?')) deleteMutation.mutate() }}
              className="p-1.5 rounded-md text-slate-400 hover:text-red-400 hover:bg-slate-800 transition-colors"
            >
              <Trash2 className="w-4 h-4" />
            </button>
            <button onClick={onClose} className="p-1.5 rounded-md text-slate-400 hover:text-white hover:bg-slate-800 transition-colors">
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto">
          <div className="p-5 space-y-4">
            {/* Meta */}
            <div className="flex flex-wrap items-center gap-2">
              {editing ? (
                <>
                  <select value={editForm.priority} onChange={e => setEditForm(f => ({ ...f, priority: e.target.value as TaskPriority }))}
                    className="bg-slate-800 border border-slate-600 rounded px-2 py-1 text-sm text-white outline-none">
                    {PRIORITIES.map(pr => <option key={pr} value={pr}>{pr}</option>)}
                  </select>
                  <select value={editForm.assigned_role} onChange={e => setEditForm(f => ({ ...f, assigned_role: e.target.value as AgentRole | '' }))}
                    className="bg-slate-800 border border-slate-600 rounded px-2 py-1 text-sm text-white outline-none">
                    <option value="">No role</option>
                    {ROLES.map(r => <option key={r} value={r}>{r}</option>)}
                  </select>
                </>
              ) : (
                <>
                  <span className={`inline-flex items-center gap-1 text-xs rounded px-2 py-0.5 font-medium ${p.bg} ${p.text}`}>
                    <span className={`w-1.5 h-1.5 rounded-full ${p.dot}`} />{p.label}
                  </span>
                  {task.assigned_role && <AgentBadge role={task.assigned_role as AgentRole} size="md" />}
                  <span className="text-xs text-slate-500 bg-slate-800 rounded px-2 py-0.5 uppercase font-medium">
                    {task.status.replace(/_/g, ' ')}
                  </span>
                </>
              )}
              {!editing && (
                <div className="relative ml-auto">
                  <button onClick={() => setShowStatusMenu(!showStatusMenu)}
                    className="flex items-center gap-1 text-xs text-slate-400 hover:text-white bg-slate-800 hover:bg-slate-700 rounded px-2 py-1 transition-colors">
                    Move to… <ChevronDown className="w-3 h-3" />
                  </button>
                  {showStatusMenu && (
                    <div className="absolute right-0 top-full mt-1 bg-slate-800 border border-slate-700 rounded-lg shadow-xl z-10 py-1 min-w-[9rem]">
                      {STATUSES.filter(s => s !== task.status).map(s => (
                        <button key={s} onClick={() => handleStatusChange(s)}
                          className="block w-full text-left px-3 py-1.5 text-sm text-slate-300 hover:text-white hover:bg-slate-700 capitalize">
                          {s.replace(/_/g, ' ')}
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              )}
            </div>

            {/* Description */}
            <div>
              <label className="text-xs text-slate-500 uppercase font-medium block mb-1">Description</label>
              {editing ? (
                <textarea value={editForm.description} onChange={e => setEditForm(f => ({ ...f, description: e.target.value }))}
                  rows={3} className="w-full bg-slate-800 border border-slate-600 rounded-lg px-3 py-2 text-sm text-slate-200 outline-none focus:border-indigo-500 resize-none" />
              ) : (
                <p className="text-sm text-slate-300 leading-relaxed">
                  {task.description ?? <span className="text-slate-600 italic">No description</span>}
                </p>
              )}
            </div>

            {/* Tags */}
            <div>
              <label className="text-xs text-slate-500 uppercase font-medium block mb-1">Tags</label>
              {editing ? (
                <input value={editForm.tags} onChange={e => setEditForm(f => ({ ...f, tags: e.target.value }))}
                  placeholder="auth, backend (comma-separated)"
                  className="w-full bg-slate-800 border border-slate-600 rounded-lg px-3 py-1.5 text-sm text-slate-200 outline-none focus:border-indigo-500" />
              ) : (
                <div className="flex flex-wrap gap-1">
                  {task.tags.length > 0
                    ? task.tags.map(tag => <span key={tag} className="text-xs bg-slate-800 text-slate-400 rounded px-2 py-0.5">{tag}</span>)
                    : <span className="text-slate-600 text-sm italic">None</span>}
                </div>
              )}
            </div>

            {task.assigned_agent_id && (
              <div>
                <label className="text-xs text-slate-500 uppercase font-medium block mb-1">Assigned Agent</label>
                <p className="text-sm font-mono text-slate-300">{task.assigned_agent_id}</p>
              </div>
            )}

            {task.output_path && (
              <div>
                <label className="text-xs text-slate-500 uppercase font-medium block mb-1">Output Path</label>
                <div className="flex items-center gap-2 bg-slate-800 rounded-lg px-3 py-2">
                  <code className="text-sm text-emerald-400 flex-1 font-mono truncate">{task.output_path}</code>
                  <button onClick={() => navigator.clipboard.writeText(task.output_path!)}
                    className="text-slate-500 hover:text-white transition-colors">
                    <Copy className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>
            )}

            {editing && (
              <div className="flex gap-2 pt-2">
                <button onClick={handleSave} disabled={updateMutation.isPending}
                  className="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg text-sm font-medium transition-colors disabled:opacity-50">
                  Save Changes
                </button>
                <button onClick={() => setEditing(false)}
                  className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-lg text-sm font-medium transition-colors">
                  Cancel
                </button>
              </div>
            )}

            {/* Activity log */}
            {task.activity && task.activity.length > 0 && (
              <div>
                <label className="text-xs text-slate-500 uppercase font-medium block mb-2">Activity</label>
                <div className="space-y-2">
                  {task.activity.map(entry => (
                    <div key={entry.id} className="flex gap-3 text-xs">
                      <span className="text-slate-600 font-mono shrink-0 pt-0.5">
                        {formatDistanceToNow(new Date(entry.timestamp), { addSuffix: true })}
                      </span>
                      <div>
                        <span className="text-slate-400">{entry.agent_id ?? 'System'}</span>
                        {entry.agent_role && <span className="text-slate-600 ml-1">({entry.agent_role})</span>}
                        <span className="text-slate-500 mx-1">·</span>
                        <span className="text-slate-300">{entry.action.replace(/_/g, ' ')}</span>
                        {entry.detail && <p className="text-slate-500 mt-0.5">{entry.detail}</p>}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
