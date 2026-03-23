import { useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { createTask } from '../api'
import { TaskPriority, AgentRole } from '../types'
import { X, PlusCircle } from 'lucide-react'

interface CreateTaskModalProps {
  onClose: () => void
}

export default function CreateTaskModal({ onClose }: CreateTaskModalProps) {
  const queryClient = useQueryClient()
  const [form, setForm] = useState({
    title: '',
    description: '',
    priority: 'medium' as TaskPriority,
    assigned_role: '' as AgentRole | '',
    tags: '',
  })

  const mutation = useMutation({
    mutationFn: () => createTask({
      title: form.title,
      description: form.description || undefined,
      priority: form.priority,
      assigned_role: (form.assigned_role as AgentRole) || undefined,
      tags: form.tags ? form.tags.split(',').map(t => t.trim()).filter(Boolean) : [],
    }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tasks'] })
      queryClient.invalidateQueries({ queryKey: ['stats'] })
      onClose()
    },
  })

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (!form.title.trim()) return
    mutation.mutate()
  }

  return (
    <div
      className="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-4"
      onClick={e => e.target === e.currentTarget && onClose()}
    >
      <div className="bg-slate-900 border border-slate-700 rounded-xl w-full max-w-md shadow-2xl">
        <div className="flex items-center justify-between p-5 border-b border-slate-800">
          <div className="flex items-center gap-2">
            <PlusCircle className="w-4 h-4 text-indigo-400" />
            <h2 className="font-semibold text-white">New Task</h2>
          </div>
          <button onClick={onClose} className="p-1.5 rounded-md text-slate-400 hover:text-white hover:bg-slate-800 transition-colors">
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="p-5 space-y-4">
          <div>
            <label className="text-xs text-slate-500 uppercase font-medium block mb-1">Title *</label>
            <input value={form.title} onChange={e => setForm(f => ({ ...f, title: e.target.value }))}
              placeholder="Task title…" required
              className="w-full bg-slate-800 border border-slate-600 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-indigo-500 placeholder:text-slate-600" />
          </div>

          <div>
            <label className="text-xs text-slate-500 uppercase font-medium block mb-1">Description</label>
            <textarea value={form.description} onChange={e => setForm(f => ({ ...f, description: e.target.value }))}
              placeholder="Describe the task…" rows={3}
              className="w-full bg-slate-800 border border-slate-600 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-indigo-500 placeholder:text-slate-600 resize-none" />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="text-xs text-slate-500 uppercase font-medium block mb-1">Priority</label>
              <select value={form.priority} onChange={e => setForm(f => ({ ...f, priority: e.target.value as TaskPriority }))}
                className="w-full bg-slate-800 border border-slate-600 rounded-lg px-3 py-2 text-sm text-white outline-none">
                <option value="low">Low</option>
                <option value="medium">Medium</option>
                <option value="high">High</option>
                <option value="critical">Critical</option>
              </select>
            </div>
            <div>
              <label className="text-xs text-slate-500 uppercase font-medium block mb-1">Assign to</label>
              <select value={form.assigned_role} onChange={e => setForm(f => ({ ...f, assigned_role: e.target.value as AgentRole | '' }))}
                className="w-full bg-slate-800 border border-slate-600 rounded-lg px-3 py-2 text-sm text-white outline-none">
                <option value="">No role</option>
                <option value="coder">Coder</option>
                <option value="reviewer">Reviewer</option>
                <option value="tester">Tester</option>
                <option value="docs_writer">Docs Writer</option>
              </select>
            </div>
          </div>

          <div>
            <label className="text-xs text-slate-500 uppercase font-medium block mb-1">Tags</label>
            <input value={form.tags} onChange={e => setForm(f => ({ ...f, tags: e.target.value }))}
              placeholder="auth, backend (comma-separated)"
              className="w-full bg-slate-800 border border-slate-600 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-indigo-500 placeholder:text-slate-600" />
          </div>

          <div className="flex gap-2 pt-1">
            <button type="submit" disabled={!form.title.trim() || mutation.isPending}
              className="flex-1 px-4 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg text-sm font-medium transition-colors disabled:opacity-50">
              {mutation.isPending ? 'Creating…' : 'Create Task'}
            </button>
            <button type="button" onClick={onClose}
              className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-lg text-sm font-medium transition-colors">
              Cancel
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
