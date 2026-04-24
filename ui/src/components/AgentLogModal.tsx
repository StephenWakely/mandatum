import { useEffect, useRef, useState } from 'react'
import { fetchAgentLog } from '../api'
import { AgentLogLine } from '../types'
import { X, Terminal } from 'lucide-react'
import { format } from 'date-fns'

interface AgentLogModalProps {
  agentId: string
  liveLogs: AgentLogLine[]
  onClose: () => void
}

export default function AgentLogModal({ agentId, liveLogs, onClose }: AgentLogModalProps) {
  const [historicalLines, setHistoricalLines] = useState<string[]>([])
  const [loading, setLoading] = useState(true)
  const bottomRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    fetchAgentLog(agentId).then(lines => {
      setHistoricalLines(lines)
      setLoading(false)
    })
  }, [agentId])

  // Auto-scroll to bottom when new lines arrive
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [historicalLines, liveLogs])

  // Deduplicate: live logs may overlap with historical; show historical first,
  // then only live lines that arrived after history was loaded.
  const historicalSet = new Set(historicalLines)
  const freshLiveLines = liveLogs.filter(l => !historicalSet.has(l.line))

  function ts(isoStr: string) {
    try {
      return format(new Date(isoStr), 'HH:mm:ss')
    } catch {
      return ''
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70">
      <div className="flex flex-col w-[780px] max-w-[95vw] h-[70vh] bg-slate-950 rounded-xl border border-slate-700 shadow-2xl overflow-hidden">
        {/* Header */}
        <div className="flex items-center gap-2 px-4 py-3 border-b border-slate-800 shrink-0">
          <Terminal className="w-4 h-4 text-emerald-400" />
          <span className="text-sm font-semibold text-white font-mono">Log: {agentId}</span>
          <button
            onClick={onClose}
            className="ml-auto p-1 rounded text-slate-500 hover:text-white hover:bg-slate-800 transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Log output */}
        <div className="flex-1 overflow-y-auto p-4 font-mono text-xs leading-relaxed">
          {loading && (
            <p className="text-slate-600">Loading log…</p>
          )}
          {!loading && historicalLines.length === 0 && liveLogs.length === 0 && (
            <p className="text-slate-600">No log output yet.</p>
          )}
          {historicalLines.map((line, i) => (
            <div key={`h-${i}`} className="flex gap-3 text-slate-300 hover:bg-slate-900/50 px-1 rounded">
              <span className="text-slate-600 shrink-0 select-none">—</span>
              <span className="break-all">{line}</span>
            </div>
          ))}
          {freshLiveLines.map((entry, i) => (
            <div key={`l-${i}`} className="flex gap-3 text-emerald-300 hover:bg-slate-900/50 px-1 rounded">
              <span className="text-slate-600 shrink-0 select-none">{ts(entry.ts)}</span>
              <span className="break-all">{entry.line}</span>
            </div>
          ))}
          <div ref={bottomRef} />
        </div>
      </div>
    </div>
  )
}
