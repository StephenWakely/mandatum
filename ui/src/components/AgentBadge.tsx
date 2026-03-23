import { AgentRole } from '../types'

export const ROLE_CONFIG: Record<AgentRole, { label: string; bg: string; text: string }> = {
  coder:       { label: 'Coder',    bg: 'bg-blue-900/60',   text: 'text-blue-300'   },
  reviewer:    { label: 'Reviewer', bg: 'bg-purple-900/60', text: 'text-purple-300' },
  tester:      { label: 'Tester',   bg: 'bg-amber-900/60',  text: 'text-amber-300'  },
  docs_writer: { label: 'Docs',     bg: 'bg-green-900/60',  text: 'text-green-300'  },
}

interface AgentBadgeProps {
  role: AgentRole
  size?: 'sm' | 'md'
}

export default function AgentBadge({ role, size = 'sm' }: AgentBadgeProps) {
  const cfg = ROLE_CONFIG[role]
  if (!cfg) return null
  return (
    <span className={`inline-flex items-center rounded px-1.5 py-0.5 font-medium ${cfg.bg} ${cfg.text} ${size === 'sm' ? 'text-xs' : 'text-sm'}`}>
      {cfg.label}
    </span>
  )
}
