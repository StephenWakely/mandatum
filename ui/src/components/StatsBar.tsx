import { useQuery } from '@tanstack/react-query'
import { fetchStats } from '../api'

const STATUS_ORDER = ['backlog', 'in_progress', 'in_review', 'testing', 'docs_needed', 'done', 'blocked']
const STATUS_COLORS: Record<string, string> = {
  backlog:     'text-slate-400',
  in_progress: 'text-blue-400',
  in_review:   'text-purple-400',
  testing:     'text-amber-400',
  docs_needed: 'text-green-400',
  done:        'text-emerald-400',
  blocked:     'text-red-400',
}

export default function StatsBar() {
  const { data: stats } = useQuery({
    queryKey: ['stats'],
    queryFn: fetchStats,
    refetchInterval: 15_000,
  })

  if (!stats) return null

  return (
    <div className="flex items-center gap-4 px-6 py-2 border-b border-slate-800 bg-slate-950 shrink-0 overflow-x-auto">
      <div className="flex items-center gap-1.5 shrink-0">
        <span className="text-slate-500 text-xs uppercase font-medium">Total</span>
        <span className="text-white font-semibold text-sm">{stats.total}</span>
      </div>
      <div className="w-px h-4 bg-slate-800 shrink-0" />
      <div className="flex items-center gap-3 flex-wrap">
        {STATUS_ORDER.map(status => {
          const count = stats.by_status[status] ?? 0
          if (count === 0) return null
          return (
            <div key={status} className="flex items-center gap-1 shrink-0">
              <span className={`text-xs font-medium ${STATUS_COLORS[status] ?? 'text-slate-400'}`}>
                {status.replace(/_/g, ' ')}
              </span>
              <span className="text-xs text-slate-500">{count}</span>
            </div>
          )
        })}
      </div>
    </div>
  )
}
