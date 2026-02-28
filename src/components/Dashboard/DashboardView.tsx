import { useMemo } from 'react';
import { BarChart3, CheckCircle2, AlertTriangle, Clock, Flame, ListTodo } from 'lucide-react';
import type { TodoItem } from '../../types/todo';
import type { IdeaItem } from '../../types/idea';
import useCountUp from '../../hooks/useCountUp';

interface DashboardViewProps {
  todos: TodoItem[];
  ideas: IdeaItem[];
}

const StatCard = ({ icon, value, suffix, label, danger }: {
  icon: React.ReactNode;
  value: number;
  suffix?: string;
  label: string;
  danger?: boolean;
}) => {
  const animated = useCountUp(value);
  return (
    <div className="stat-card">
      <div className={`stat-icon${danger ? ' stat-icon-danger' : ''}`}>{icon}</div>
      <div className="stat-value">{animated}{suffix}</div>
      <div className="stat-label">{label}</div>
    </div>
  );
};

const FractionCard = ({ icon, numerator, denominator, label }: {
  icon: React.ReactNode;
  numerator: number;
  denominator: number;
  label: string;
}) => {
  const n = useCountUp(numerator);
  const d = useCountUp(denominator);
  return (
    <div className="stat-card">
      <div className="stat-icon">{icon}</div>
      <div className="stat-value">{n}/{d}</div>
      <div className="stat-label">{label}</div>
    </div>
  );
};

const getWeekStart = () => {
  const now = new Date();
  const day = now.getDay();
  const monday = new Date(now);
  monday.setHours(0, 0, 0, 0);
  monday.setDate(now.getDate() - day + (day === 0 ? -6 : 1));
  return monday.getTime();
};

const getStreak = (todos: TodoItem[]) => {
  const days = new Set<string>();
  for (const t of todos) {
    if (!t.completed) continue;
    const d = new Date(t.createdAt);
    days.add(`${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`);
  }

  let streak = 0;
  const today = new Date();
  today.setHours(0, 0, 0, 0);

  for (let i = 0; i < 365; i++) {
    const d = new Date(today.getTime() - i * 86400000);
    const key = `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
    if (days.has(key)) {
      streak++;
    } else if (i > 0) {
      break;
    }
  }

  return streak;
};

const priorities = ['high', 'medium', 'low'] as const;

const DashboardView = ({ todos, ideas }: DashboardViewProps) => {
  const stats = useMemo(() => {
    const now = Date.now();
    const weekStart = getWeekStart();

    const active = todos.filter((t) => !t.completed);
    const completed = todos.filter((t) => t.completed);
    const overdue = active.filter((t) => t.dueAt && Date.parse(t.dueAt) <= now);
    const doneThisWeek = completed.filter((t) => Date.parse(t.createdAt) >= weekStart);
    const rate = todos.length > 0 ? Math.round((completed.length / todos.length) * 100) : 0;

    const tagCounts: Record<string, number> = {};
    for (const item of [...todos, ...ideas]) {
      for (const tag of item.tags) {
        tagCounts[tag] = (tagCounts[tag] || 0) + 1;
      }
    }

    const topTags = Object.entries(tagCounts).sort((a, b) => b[1] - a[1]).slice(0, 8);

    const byPriority: Record<string, number> = { high: 0, medium: 0, low: 0 };
    for (const t of active) {
      if (t.priority in byPriority) byPriority[t.priority]++;
    }

    return {
      total: todos.length,
      active: active.length,
      completed: completed.length,
      overdue: overdue.length,
      doneThisWeek: doneThisWeek.length,
      rate,
      streak: getStreak(todos),
      topTags,
      byPriority,
    };
  }, [todos, ideas]);

  return (
    <section className="stack">
      <header className="section-header">
        <div className="section-badge">DASHBOARD</div>
        <h1 className="section-title">Dashboard</h1>
        <p className="section-subtitle">Overview of your productivity and task statistics.</p>
      </header>

      <div className="dashboard-grid">
        <StatCard icon={<ListTodo size={20} />} value={stats.active} label="Active tasks" />
        <StatCard icon={<CheckCircle2 size={20} />} value={stats.doneThisWeek} label="Done this week" />
        <StatCard icon={<AlertTriangle size={20} />} value={stats.overdue} label="Overdue" danger />
        <StatCard icon={<Flame size={20} />} value={stats.streak} label="Day streak" />
        <StatCard icon={<BarChart3 size={20} />} value={stats.rate} suffix="%" label="Completion rate" />
        <FractionCard icon={<Clock size={20} />} numerator={stats.completed} denominator={stats.total} label="Done / total" />
      </div>

      <article className="panel">
        <h2 className="panel-title">Active by priority</h2>
        <div className="priority-bars">
          {priorities.map((p) => (
            <div key={p} className="priority-bar-row">
              <span className={`priority-bar-label priority-${p}`}>{p[0].toUpperCase() + p.slice(1)}</span>
              <div className="priority-bar">
                <div
                  className={`priority-bar-fill priority-bar-${p}`}
                  style={{ width: stats.active > 0 ? `${(stats.byPriority[p] / stats.active) * 100}%` : '0%' }}
                />
              </div>
              <span className="priority-bar-count">{stats.byPriority[p]}</span>
            </div>
          ))}
        </div>
      </article>

      {stats.topTags.length > 0 && (
        <article className="panel">
          <h2 className="panel-title">Top tags</h2>
          <div className="todo-tags">
            {stats.topTags.map(([tag, count]) => (
              <span key={tag} className="tag tag-display">
                {tag} <span className="tag-count">({count})</span>
              </span>
            ))}
          </div>
        </article>
      )}
    </section>
  );
};

export default DashboardView;
