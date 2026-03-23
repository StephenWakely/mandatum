export type TaskStatus = 'backlog' | 'in_progress' | 'in_review' | 'testing' | 'docs_needed' | 'done' | 'blocked'
export type TaskPriority = 'low' | 'medium' | 'high' | 'critical'
export type AgentRole = 'coder' | 'reviewer' | 'tester' | 'docs_writer'

export interface Task {
  id: string
  title: string
  description?: string
  status: TaskStatus
  assigned_role?: AgentRole
  assigned_agent_id?: string
  priority: TaskPriority
  created_at: string
  updated_at: string
  output_path?: string
  tags: string[]
  activity?: ActivityEntry[]
}

export interface ActivityEntry {
  id: string
  task_id: string
  agent_id?: string
  agent_role?: AgentRole
  action: string
  detail?: string
  timestamp: string
}

export interface Agent {
  agent_id: string
  role: AgentRole
  last_seen?: string
  current_task_id?: string
}

export interface Stats {
  total: number
  by_status: Record<string, number>
  by_role: Record<string, number>
}
