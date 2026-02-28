import { useMemo, useState } from 'react';
import { Check, Clock3, Filter, Pencil, Save, Search, Trash2, X } from 'lucide-react';
import type { NewTodoInput, Priority, TodoItem } from '../../types/todo';
import type { TeamMember, TeamRole } from '../../types/team';
import TagInput from '../TagInput';
import Select from '../Select';

interface TodoViewProps {
  todos: TodoItem[];
  onAddTodo: (input: NewTodoInput) => boolean;
  onToggleComplete: (todoId: string) => void;
  onDeleteTodo: (todoId: string) => void;
  onUpdateTodo: (todo: TodoItem) => void;
  teamMode: boolean;
  teamMembers: TeamMember[];
  currentMemberId: string | null;
  currentMemberRole: TeamRole | null;
  canManageTodo: (todo: TodoItem) => boolean;
}

const formatDate = (value: string | null) => {
  if (!value) return 'not set';

  return new Date(value).toLocaleString('en-US', {
    dateStyle: 'medium',
    timeStyle: 'short',
  });
};

const toLocalDatetime = (iso: string | null) => {
  if (!iso) return '';
  const d = new Date(iso);
  const offset = d.getTimezoneOffset();
  const local = new Date(d.getTime() - offset * 60000);
  return local.toISOString().slice(0, 16);
};

const priorityLabel: Record<Priority, string> = { low: 'Low', medium: 'Medium', high: 'High' };
const priorityOrder: Record<Priority, number> = { high: 0, medium: 1, low: 2 };

const prioritySelectOptions = [
  { value: 'low', label: 'Low' },
  { value: 'medium', label: 'Medium' },
  { value: 'high', label: 'High' },
];
const roleLabel: Record<TeamRole, string> = {
  owner: 'Owner',
  admin: 'Admin',
  member: 'Member',
};

type StatusFilter = 'all' | 'active' | 'done' | 'overdue';
const statusOptions: StatusFilter[] = ['all', 'active', 'done', 'overdue'];
const priorityOptions = ['all', 'high', 'medium', 'low'] as const;

const TodoView = ({
  todos,
  onAddTodo,
  onToggleComplete,
  onDeleteTodo,
  onUpdateTodo,
  teamMode,
  teamMembers,
  currentMemberId,
  currentMemberRole,
  canManageTodo,
}: TodoViewProps) => {
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [remindAtLocal, setRemindAtLocal] = useState('');
  const [dueAtLocal, setDueAtLocal] = useState('');
  const [priority, setPriority] = useState<Priority>('medium');
  const [tags, setTags] = useState<string[]>([]);
  const [assigneeMemberId, setAssigneeMemberId] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editState, setEditState] = useState<Partial<TodoItem & { dueAtLocal: string; remindAtLocal: string }>>({});
  const [searchQuery, setSearchQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all');
  const [priorityFilter, setPriorityFilter] = useState<Priority | 'all'>('all');
  const [tagFilter, setTagFilter] = useState<string | null>(null);
  const [showFilters, setShowFilters] = useState(false);

  const allTags = useMemo(() => {
    const set = new Set<string>();
    for (const todo of todos) {
      for (const tag of todo.tags) set.add(tag);
    }
    return Array.from(set).sort();
  }, [todos]);

  const memberNameById = useMemo(() => {
    const map = new Map<string, string>();
    for (const member of teamMembers) {
      map.set(member.id, member.name);
    }
    return map;
  }, [teamMembers]);

  const assigneeOptions = useMemo(
    () => [
      { value: '', label: 'Unassigned' },
      ...teamMembers.map((member) => ({
        value: member.id,
        label: `${member.name} (${roleLabel[member.role]})`,
      })),
    ],
    [teamMembers]
  );

  const filteredAndSorted = useMemo(() => {
    const now = Date.now();
    const query = searchQuery.toLowerCase().trim();

    return [...todos]
      .filter((todo) => {
        if (statusFilter === 'active' && todo.completed) return false;
        if (statusFilter === 'done' && !todo.completed) return false;
        if (statusFilter === 'overdue') {
          if (todo.completed || !todo.dueAt || Date.parse(todo.dueAt) > now) return false;
        }
        if (priorityFilter !== 'all' && todo.priority !== priorityFilter) return false;
        if (tagFilter && !todo.tags.includes(tagFilter)) return false;
        if (query) {
          const inTitle = todo.title.toLowerCase().includes(query);
          const inDesc = todo.description.toLowerCase().includes(query);
          const inTags = todo.tags.some((t) => t.toLowerCase().includes(query));
          if (!inTitle && !inDesc && !inTags) return false;
        }
        return true;
      })
      .sort((left, right) => {
        if (left.completed !== right.completed) {
          return left.completed ? 1 : -1;
        }

        const lp = priorityOrder[left.priority] ?? 1;
        const rp = priorityOrder[right.priority] ?? 1;
        if (lp !== rp) return lp - rp;

        const leftDue = left.dueAt ? Date.parse(left.dueAt) : Number.MAX_SAFE_INTEGER;
        const rightDue = right.dueAt ? Date.parse(right.dueAt) : Number.MAX_SAFE_INTEGER;

        if (leftDue !== rightDue) {
          return leftDue - rightDue;
        }

        return Date.parse(right.createdAt) - Date.parse(left.createdAt);
      });
  }, [todos, searchQuery, statusFilter, priorityFilter, tagFilter]);

  const hasActiveFilters = statusFilter !== 'all' || priorityFilter !== 'all' || tagFilter !== null;

  const handleSubmit = (event: React.FormEvent) => {
    event.preventDefault();

    const created = onAddTodo({
      title,
      description,
      remindAtLocal,
      dueAtLocal,
      priority,
      tags,
      assigneeMemberId: teamMode ? assigneeMemberId : null,
    });

    if (!created) return;

    setTitle('');
    setDescription('');
    setRemindAtLocal('');
    setDueAtLocal('');
    setPriority('medium');
    setTags([]);
    setAssigneeMemberId(null);
  };

  const startEdit = (todo: TodoItem) => {
    setEditingId(todo.id);
    setEditState({
      title: todo.title,
      description: todo.description,
      priority: todo.priority,
      tags: [...todo.tags],
      dueAtLocal: toLocalDatetime(todo.dueAt),
      remindAtLocal: toLocalDatetime(todo.remindAt),
      assigneeMemberId: todo.assigneeMemberId,
    });
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditState({});
  };

  const saveEdit = (todo: TodoItem) => {
    const dueVal = editState.dueAtLocal?.trim();
    const remindVal = editState.remindAtLocal?.trim();
    const dueAt = dueVal ? new Date(dueVal).toISOString() : null;
    const remindAt = remindVal ? new Date(remindVal).toISOString() : null;
    const nextAssigneeId = teamMode ? editState.assigneeMemberId?.trim() || null : null;
    const nextAssigneeName = nextAssigneeId ? memberNameById.get(nextAssigneeId) ?? null : null;

    onUpdateTodo({
      ...todo,
      title: editState.title?.trim() || todo.title,
      description: editState.description?.trim() ?? todo.description,
      priority: (editState.priority as Priority) || todo.priority,
      tags: editState.tags || todo.tags,
      dueAt,
      remindAt,
      dueFiredAt: dueAt !== todo.dueAt ? null : todo.dueFiredAt,
      reminderFiredAt: remindAt !== todo.remindAt ? null : todo.reminderFiredAt,
      assigneeMemberId: nextAssigneeId,
      assigneeMemberName: nextAssigneeName,
    });

    setEditingId(null);
    setEditState({});
  };

  return (
    <section className="stack">
      <header className="section-header">
        <div className="section-badge">TASKS</div>
        <h1 className="section-title">Todo Planner</h1>
        <p className="section-subtitle">
          Add tasks, set reminder time, and set a due time. Flux Flow will notify you when it is time.
        </p>
      </header>

      <article className="panel">
        <h2 className="panel-title">New TODO</h2>
        <form className="form-grid" onSubmit={handleSubmit}>
          <label className="field">
            <span>Task title</span>
            <input
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              placeholder="e.g. finish presentation"
              maxLength={120}
              required
            />
          </label>

          <label className="field">
            <span>Notes (optional)</span>
            <textarea
              value={description}
              onChange={(event) => setDescription(event.target.value)}
              placeholder="Add details..."
              rows={3}
              maxLength={420}
            />
          </label>

          <div className="field">
            <span>Priority</span>
            <Select value={priority} options={prioritySelectOptions} onChange={(v) => setPriority(v as Priority)} />
          </div>

          {teamMode && (
            <div className="field">
              <span>Assignee</span>
              <Select
                value={assigneeMemberId ?? ''}
                options={assigneeOptions}
                onChange={(value) => setAssigneeMemberId(value || null)}
              />
            </div>
          )}

          <div className="field-row">
            <label className="field">
              <span>Remind at</span>
              <input
                type="datetime-local"
                value={remindAtLocal}
                onChange={(event) => setRemindAtLocal(event.target.value)}
              />
            </label>
            <label className="field">
              <span>Due at</span>
              <input
                type="datetime-local"
                value={dueAtLocal}
                onChange={(event) => setDueAtLocal(event.target.value)}
              />
            </label>
          </div>

          <div className="field">
            <span>Tags</span>
            <TagInput tags={tags} onChange={setTags} />
          </div>

          <button type="submit" className="btn btn-primary">
            Add TODO
          </button>
        </form>
      </article>

      <article className="panel">
        <div className="panel-header-row">
          <h2 className="panel-title" style={{ marginBottom: 0 }}>Task List</h2>
          <span className="filter-count">{filteredAndSorted.length} of {todos.length}</span>
        </div>

        <div className="search-bar">
          <Search size={16} className="search-icon" />
          <input
            className="search-input"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search tasks..."
          />
          {searchQuery && (
            <button type="button" className="search-clear" onClick={() => setSearchQuery('')}>
              <X size={14} />
            </button>
          )}
          <button
            type="button"
            className={`btn btn-ghost btn-filter-toggle ${showFilters || hasActiveFilters ? 'active' : ''}`}
            onClick={() => setShowFilters((v) => !v)}
          >
            <Filter size={15} />
            Filters
          </button>
        </div>

        {showFilters && (
          <div className="filter-bar">
            <div className="filter-group">
              <span className="filter-label">Status</span>
              <div className="filter-chips">
                {statusOptions.map((s) => (
                  <button
                    key={s}
                    type="button"
                    className={`filter-chip ${statusFilter === s ? 'active' : ''}`}
                    onClick={() => setStatusFilter(s)}
                  >
                    {s[0].toUpperCase() + s.slice(1)}
                  </button>
                ))}
              </div>
            </div>
            <div className="filter-group">
              <span className="filter-label">Priority</span>
              <div className="filter-chips">
                {priorityOptions.map((p) => (
                  <button
                    key={p}
                    type="button"
                    className={`filter-chip ${priorityFilter === p ? 'active' : ''}`}
                    onClick={() => setPriorityFilter(p)}
                  >
                    {p === 'all' ? 'All' : priorityLabel[p]}
                  </button>
                ))}
              </div>
            </div>
            {allTags.length > 0 && (
              <div className="filter-group">
                <span className="filter-label">Tag</span>
                <div className="filter-chips">
                  <button
                    type="button"
                    className={`filter-chip ${tagFilter === null ? 'active' : ''}`}
                    onClick={() => setTagFilter(null)}
                  >
                    All
                  </button>
                  {allTags.map((t) => (
                    <button
                      key={t}
                      type="button"
                      className={`filter-chip ${tagFilter === t ? 'active' : ''}`}
                      onClick={() => setTagFilter(t)}
                    >
                      {t}
                    </button>
                  ))}
                </div>
              </div>
            )}
            {hasActiveFilters && (
              <button
                type="button"
                className="btn btn-ghost"
                onClick={() => { setStatusFilter('all'); setPriorityFilter('all'); setTagFilter(null); }}
              >
                <X size={14} />
                Clear filters
              </button>
            )}
          </div>
        )}

        {filteredAndSorted.length === 0 && (
          <div className="empty-state">
            <Clock3 size={20} />
            <p>{todos.length === 0 ? 'You do not have any TODO items yet.' : 'No tasks match your search or filters.'}</p>
          </div>
        )}

        <div className="todo-list">
          {filteredAndSorted.map((todo) => {
            const overdue = !todo.completed && todo.dueAt !== null && Date.parse(todo.dueAt) <= Date.now();
            const isEditing = editingId === todo.id;

            return (
              <div key={todo.id} className={`todo-card ${todo.completed ? 'done' : ''}`}>
                {isEditing ? (
                  <div className="form-grid">
                    <label className="field">
                      <span>Title</span>
                      <input
                        value={editState.title ?? ''}
                        onChange={(e) => setEditState((s) => ({ ...s, title: e.target.value }))}
                        maxLength={120}
                      />
                    </label>
                    <label className="field">
                      <span>Notes</span>
                      <textarea
                        value={editState.description ?? ''}
                        onChange={(e) => setEditState((s) => ({ ...s, description: e.target.value }))}
                        rows={2}
                        maxLength={420}
                      />
                    </label>
                    <div className="field">
                      <span>Priority</span>
                      <Select
                        value={editState.priority ?? 'medium'}
                        options={prioritySelectOptions}
                        onChange={(v) => setEditState((s) => ({ ...s, priority: v as Priority }))}
                      />
                    </div>
                    {teamMode && (
                      <div className="field">
                        <span>Assignee</span>
                        <Select
                          value={(editState.assigneeMemberId as string | null) ?? ''}
                          options={assigneeOptions}
                          onChange={(value) => setEditState((s) => ({ ...s, assigneeMemberId: value || null }))}
                        />
                      </div>
                    )}
                    <div className="field-row">
                      <label className="field">
                        <span>Remind at</span>
                        <input
                          type="datetime-local"
                          value={editState.remindAtLocal ?? ''}
                          onChange={(e) => setEditState((s) => ({ ...s, remindAtLocal: e.target.value }))}
                        />
                      </label>
                      <label className="field">
                        <span>Due at</span>
                        <input
                          type="datetime-local"
                          value={editState.dueAtLocal ?? ''}
                          onChange={(e) => setEditState((s) => ({ ...s, dueAtLocal: e.target.value }))}
                        />
                      </label>
                    </div>
                    <div className="field">
                      <span>Tags</span>
                      <TagInput
                        tags={editState.tags || []}
                        onChange={(t) => setEditState((s) => ({ ...s, tags: t }))}
                      />
                    </div>
                    <div className="todo-actions">
                      <button type="button" className="btn btn-primary" onClick={() => saveEdit(todo)}>
                        <Save size={15} />
                        Save
                      </button>
                      <button type="button" className="btn btn-ghost" onClick={cancelEdit}>
                        <X size={15} />
                        Cancel
                      </button>
                    </div>
                  </div>
                ) : (
                  <>
                    <div className="todo-top">
                      <div>
                        <h3 className="todo-title">{todo.title}</h3>
                        {todo.description && <p className="todo-description">{todo.description}</p>}
                      </div>
                      <div className="todo-top-right">
                        <span className={`todo-priority priority-${todo.priority}`}>
                          {priorityLabel[todo.priority]}
                        </span>
                        <span className={`todo-status ${todo.completed ? 'done' : ''}`}>
                          {todo.completed ? 'Done' : 'Active'}
                        </span>
                      </div>
                    </div>

                    {todo.tags.length > 0 && (
                      <div className="todo-tags">
                        {todo.tags.map((tag) => (
                          <span key={tag} className="tag tag-display">{tag}</span>
                        ))}
                      </div>
                    )}

                    <div className="todo-meta">
                      <span className="todo-pill">Reminder: {formatDate(todo.remindAt)}</span>
                      <span className={`todo-pill ${overdue ? 'danger' : ''}`}>
                        Due: {formatDate(todo.dueAt)}
                      </span>
                      {teamMode && (
                        <span className="todo-pill">
                          Assignee: {todo.assigneeMemberName ?? 'Unassigned'}
                        </span>
                      )}
                      {teamMode && (
                        <span className="todo-pill">
                          By: {todo.createdByMemberName ?? 'Unknown'}
                        </span>
                      )}
                    </div>

                    <div className="todo-actions">
                      <button
                        type="button"
                        className="btn btn-ghost"
                        onClick={() => onToggleComplete(todo.id)}
                        disabled={!canManageTodo(todo)}
                        title={!canManageTodo(todo) ? 'No permission to edit this task.' : undefined}
                      >
                        <Check size={15} />
                        {todo.completed ? 'Reopen' : 'Mark done'}
                      </button>
                      <button
                        type="button"
                        className="btn btn-ghost"
                        onClick={() => startEdit(todo)}
                        disabled={!canManageTodo(todo)}
                        title={!canManageTodo(todo) ? 'No permission to edit this task.' : undefined}
                      >
                        <Pencil size={15} />
                        Edit
                      </button>
                      <button
                        type="button"
                        className="btn btn-danger"
                        onClick={() => {
                          if (window.confirm(`Delete "${todo.title}"?`)) onDeleteTodo(todo.id);
                        }}
                        disabled={!canManageTodo(todo)}
                        title={!canManageTodo(todo) ? 'No permission to delete this task.' : undefined}
                      >
                        <Trash2 size={15} />
                        Delete
                      </button>
                    </div>
                    {teamMode && (
                      <div className="idea-footer">
                        Signed in as {currentMemberId ? currentMemberRole ? roleLabel[currentMemberRole] : 'Member' : 'Guest'}
                      </div>
                    )}
                  </>
                )}
              </div>
            );
          })}
        </div>
      </article>
    </section>
  );
};

export default TodoView;
