import { useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Copy, LogOut, UserRound, Users } from 'lucide-react';
import CalendarView from './components/Calendar/CalendarView';
import DashboardView from './components/Dashboard/DashboardView';
import IdeasView from './components/Ideas/IdeasView';
import NotificationContainer from './components/NotificationContainer';
import Sidebar from './components/Sidebar';
import TeamGateView from './components/Team/TeamGateView';
import Titlebar from './components/Titlebar';
import TodoView from './components/Todo/TodoView';
import Select from './components/Select';
import { NotificationProvider, useNotification } from './contexts/NotificationContext';
import type { FlowMode } from './types/flow';
import type { IdeaItem, NewIdeaInput } from './types/idea';
import type { TeamActivityItem, TeamContext, TeamMember, TeamRole, TeamSession } from './types/team';
import type { NewTodoInput, TodoItem } from './types/todo';
import type { AppView } from './types/view';

type ReminderKind = 'reminder' | 'due';

interface ReminderAlert {
  kind: ReminderKind;
  todo: TodoItem;
}

let notificationPermissionRequested = false;
const FLOW_MODE_STORAGE_KEY = 'flux-flow:mode';
const TEAM_SESSION_STORAGE_KEY = 'flux-flow:team-session';
const teamRoleLabels: Record<TeamRole, string> = {
  owner: 'Owner',
  admin: 'Admin',
  member: 'Member',
};

const createId = () => {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID();
  }

  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
};

const parseTeamSession = (value: string | null): TeamSession | null => {
  if (!value) return null;

  try {
    const parsed = JSON.parse(value) as Partial<TeamSession>;
    if (
      typeof parsed?.teamCode === 'string' &&
      typeof parsed?.memberId === 'string' &&
      typeof parsed?.memberName === 'string' &&
      typeof parsed?.owner === 'boolean' &&
      typeof parsed?.memberCount === 'number'
    ) {
      const role: TeamRole =
        parsed.role === 'owner' || parsed.role === 'admin' || parsed.role === 'member'
          ? parsed.role
          : parsed.owner
            ? 'owner'
            : 'member';

      return {
        teamCode: parsed.teamCode,
        memberId: parsed.memberId,
        memberName: parsed.memberName,
        role,
        owner: parsed.owner,
        memberCount: parsed.memberCount,
      };
    }
  } catch {
    return null;
  }

  return null;
};

const loadInitialFlowMode = (): FlowMode => {
  if (typeof window === 'undefined') return 'personal';

  const mode = window.localStorage.getItem(FLOW_MODE_STORAGE_KEY);
  return mode === 'team' ? 'team' : 'personal';
};

const loadInitialTeamSession = (): TeamSession | null => {
  if (typeof window === 'undefined') return null;
  return parseTeamSession(window.localStorage.getItem(TEAM_SESSION_STORAGE_KEY));
};

const formatDateTime = (value: string | null) => {
  if (!value) return 'no time set';

  return new Date(value).toLocaleString('en-US', {
    dateStyle: 'medium',
    timeStyle: 'short',
  });
};

const toIsoOrNull = (value: string) => {
  if (!value.trim()) return null;

  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return null;

  return parsed.toISOString();
};

const evaluateReminders = (items: TodoItem[], nowMs: number) => {
  const alerts: ReminderAlert[] = [];
  let changed = false;

  const nextTodos = items.map((todo) => {
    if (todo.completed) {
      return todo;
    }

    let nextTodo = todo;

    if (todo.remindAt && !todo.reminderFiredAt && Date.parse(todo.remindAt) <= nowMs) {
      alerts.push({ kind: 'reminder', todo: nextTodo });
      nextTodo = { ...nextTodo, reminderFiredAt: new Date(nowMs).toISOString() };
      changed = true;
    }

    if (todo.dueAt && !todo.dueFiredAt && Date.parse(todo.dueAt) <= nowMs) {
      alerts.push({ kind: 'due', todo: nextTodo });
      nextTodo = { ...nextTodo, dueFiredAt: new Date(nowMs).toISOString() };
      changed = true;
    }

    return nextTodo;
  });

  return {
    alerts,
    nextTodos: changed ? nextTodos : items,
  };
};

const requestNotificationPermission = async () => {
  if (typeof window === 'undefined' || !('Notification' in window)) {
    return false;
  }

  if (Notification.permission === 'granted') {
    return true;
  }

  if (Notification.permission === 'denied') {
    return false;
  }

  if (notificationPermissionRequested) {
    return false;
  }

  notificationPermissionRequested = true;

  try {
    const permission = await Notification.requestPermission();
    return permission === 'granted';
  } catch {
    return false;
  }
};

const sendSystemNotification = async (title: string, body: string) => {
  if (typeof window === 'undefined' || !('Notification' in window)) {
    return;
  }

  const allowed = await requestNotificationPermission();
  if (!allowed) return;

  new Notification(title, { body });
};

const AppContent = () => {
  const { showNotification } = useNotification();
  const [currentView, setCurrentView] = useState<AppView>('dashboard');
  const [todos, setTodos] = useState<TodoItem[]>([]);
  const [ideas, setIdeas] = useState<IdeaItem[]>([]);
  const [flowMode, setFlowMode] = useState<FlowMode>(loadInitialFlowMode);
  const [teamSession, setTeamSession] = useState<TeamSession | null>(loadInitialTeamSession);
  const [teamMembers, setTeamMembers] = useState<TeamMember[]>([]);
  const [teamActivity, setTeamActivity] = useState<TeamActivityItem[]>([]);
  const [teamAction, setTeamAction] = useState<'create' | 'join' | null>(null);
  const [roleUpdateMemberId, setRoleUpdateMemberId] = useState<string | null>(null);
  const [refreshingActivity, setRefreshingActivity] = useState(false);
  const [storageReady, setStorageReady] = useState(false);
  const [loadedStorageKey, setLoadedStorageKey] = useState<string | null>(null);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const todosRef = useRef(todos);
  const activeTeamCode = flowMode === 'team' ? teamSession?.teamCode ?? null : null;
  const activeTeamMemberId = flowMode === 'team' ? teamSession?.memberId ?? null : null;
  const activeStorageKey = useMemo(
    () => (flowMode === 'personal' ? 'personal' : activeTeamCode ? `team:${activeTeamCode}` : null),
    [flowMode, activeTeamCode]
  );
  const teamMemberNameById = useMemo(() => {
    const map = new Map<string, string>();
    for (const member of teamMembers) {
      map.set(member.id, member.name);
    }
    return map;
  }, [teamMembers]);

  useEffect(() => {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(FLOW_MODE_STORAGE_KEY, flowMode);
  }, [flowMode]);

  useEffect(() => {
    if (typeof window === 'undefined') return;

    if (teamSession) {
      window.localStorage.setItem(TEAM_SESSION_STORAGE_KEY, JSON.stringify(teamSession));
    } else {
      window.localStorage.removeItem(TEAM_SESSION_STORAGE_KEY);
    }
  }, [teamSession]);

  useEffect(() => {
    let cancelled = false;

    const loadData = async () => {
      if (!activeStorageKey) {
        setTodos([]);
        setIdeas([]);
        setTeamMembers([]);
        setTeamActivity([]);
        todosRef.current = [];
        setLoadedStorageKey(null);
        setStorageReady(false);
        return;
      }

      setStorageReady(false);
      setLoadedStorageKey(null);

      try {
        let storedTodos: TodoItem[] = [];
        let storedIdeas: IdeaItem[] = [];

        if (flowMode === 'personal') {
          setTeamMembers([]);
          setTeamActivity([]);
          [storedTodos, storedIdeas] = await Promise.all([
            invoke<TodoItem[]>('load_todos'),
            invoke<IdeaItem[]>('load_ideas'),
          ]);
        } else if (activeTeamCode && activeTeamMemberId) {
          const context = await invoke<TeamContext>('load_team_context', {
            teamCode: activeTeamCode,
            memberId: activeTeamMemberId,
          });
          if (cancelled) return;

          setTeamSession(context.session);
          setTeamMembers(context.members);

          if (context.session.owner) {
            const activity = await invoke<TeamActivityItem[]>('load_team_activity', {
              teamCode: activeTeamCode,
              memberId: context.session.memberId,
            });
            if (!cancelled) {
              setTeamActivity(activity);
            }
          } else {
            setTeamActivity([]);
          }

          [storedTodos, storedIdeas] = await Promise.all([
            invoke<TodoItem[]>('load_team_todos', {
              teamCode: activeTeamCode,
              memberId: context.session.memberId,
            }),
            invoke<IdeaItem[]>('load_team_ideas', {
              teamCode: activeTeamCode,
              memberId: context.session.memberId,
            }),
          ]);
        }

        if (cancelled) return;

        const migratedTodos = storedTodos.map((todo) => ({
          ...todo,
          priority: todo.priority || 'medium',
          tags: todo.tags || [],
          createdByMemberId: todo.createdByMemberId ?? null,
          createdByMemberName: todo.createdByMemberName ?? null,
          assigneeMemberId: todo.assigneeMemberId ?? null,
          assigneeMemberName: todo.assigneeMemberName ?? null,
        }));
        const migratedIdeas = storedIdeas.map((idea) => ({
          ...idea,
          tags: idea.tags || [],
        }));

        setTodos(migratedTodos);
        setIdeas(migratedIdeas);
        todosRef.current = migratedTodos;
      } catch (error) {
        if (cancelled) return;

        console.error('Failed to load data:', error);
        setTodos([]);
        setIdeas([]);
        setTeamMembers([]);
        setTeamActivity([]);
        todosRef.current = [];
        showNotification({
          type: 'error',
          title: 'Load failed',
          message:
            flowMode === 'personal'
              ? 'Could not load personal data from your home .flux-flow folder.'
              : `Could not load team data for ${activeTeamCode ?? 'the selected team'}.`,
          duration: 6500,
        });
      } finally {
        if (!cancelled) {
          setLoadedStorageKey(activeStorageKey);
          setStorageReady(true);
        }
      }
    };

    void loadData();

    return () => {
      cancelled = true;
    };
  }, [activeStorageKey, activeTeamCode, activeTeamMemberId, flowMode, showNotification]);

  useEffect(() => {
    todosRef.current = todos;
    if (!storageReady || !activeStorageKey || loadedStorageKey !== activeStorageKey) return;

    const saveTodos = async () => {
      try {
        if (flowMode === 'personal') {
          await invoke('save_todos', { todos });
        } else if (activeTeamCode && activeTeamMemberId) {
          await invoke('save_team_todos', {
            teamCode: activeTeamCode,
            memberId: activeTeamMemberId,
            todos,
          });
        }
      } catch (error) {
        console.error('Failed to save todos:', error);
        showNotification({
          type: 'error',
          title: 'Save failed',
          message:
            flowMode === 'personal'
              ? 'Could not save personal todos.json in your home .flux-flow folder.'
              : `Could not save team TODOs for ${activeTeamCode ?? 'the current team'} (permission or backend error).`,
          duration: 6500,
        });
      }
    };

    void saveTodos();
  }, [
    todos,
    storageReady,
    showNotification,
    activeStorageKey,
    flowMode,
    activeTeamCode,
    activeTeamMemberId,
    loadedStorageKey,
  ]);

  useEffect(() => {
    if (!storageReady || !activeStorageKey || loadedStorageKey !== activeStorageKey) return;

    const saveIdeas = async () => {
      try {
        if (flowMode === 'personal') {
          await invoke('save_ideas', { ideas });
        } else if (activeTeamCode && activeTeamMemberId) {
          await invoke('save_team_ideas', {
            teamCode: activeTeamCode,
            memberId: activeTeamMemberId,
            ideas,
          });
        }
      } catch (error) {
        console.error('Failed to save ideas:', error);
        showNotification({
          type: 'error',
          title: 'Save failed',
          message:
            flowMode === 'personal'
              ? 'Could not save personal ideas.json in your home .flux-flow folder.'
              : `Could not save team ideas for ${activeTeamCode ?? 'the current team'}.`,
          duration: 6500,
        });
      }
    };

    void saveIdeas();
  }, [
    ideas,
    storageReady,
    showNotification,
    activeStorageKey,
    flowMode,
    activeTeamCode,
    activeTeamMemberId,
    loadedStorageKey,
  ]);

  useEffect(() => {
    const checkReminders = async () => {
      const { alerts, nextTodos } = evaluateReminders(todosRef.current, Date.now());
      if (alerts.length === 0) return;

      todosRef.current = nextTodos;
      setTodos(nextTodos);

      for (const alert of alerts) {
        const isDueAlert = alert.kind === 'due';
        const moment = isDueAlert ? alert.todo.dueAt : alert.todo.remindAt;
        const text = isDueAlert
          ? `Task "${alert.todo.title}" is due at ${formatDateTime(moment)}.`
          : `Reminder for task "${alert.todo.title}" (${formatDateTime(moment)}).`;

        showNotification({
          type: isDueAlert ? 'warning' : 'info',
          title: isDueAlert ? 'TODO due' : 'TODO reminder',
          message: text,
          duration: 6500,
        });

        await sendSystemNotification(isDueAlert ? 'Flux Flow: task due' : 'Flux Flow: reminder', text);
      }
    };

    void checkReminders();
    const timer = window.setInterval(() => {
      void checkReminders();
    }, 10000);

    return () => {
      window.clearInterval(timer);
    };
  }, [showNotification]);

  const canManageTodo = (todo: TodoItem) => {
    if (flowMode !== 'team' || !teamSession) return true;
    if (teamSession.role === 'owner' || teamSession.role === 'admin') return true;
    return todo.createdByMemberId === teamSession.memberId || todo.assigneeMemberId === teamSession.memberId;
  };

  const refreshTeamActivity = async () => {
    if (flowMode !== 'team' || !activeTeamCode || !activeTeamMemberId || !teamSession?.owner) return;

    setRefreshingActivity(true);
    try {
      const activity = await invoke<TeamActivityItem[]>('load_team_activity', {
        teamCode: activeTeamCode,
        memberId: activeTeamMemberId,
      });
      setTeamActivity(activity);
    } catch (error) {
      console.error('Failed to load team activity:', error);
      showNotification({
        type: 'error',
        title: 'Activity load failed',
        message: 'Could not load team activity feed.',
      });
    } finally {
      setRefreshingActivity(false);
    }
  };

  const handleUpdateMemberRole = async (targetMemberId: string, role: TeamRole) => {
    if (flowMode !== 'team' || !activeTeamCode || !activeTeamMemberId || !teamSession) return;
    if (teamSession.role !== 'owner') return;

    setRoleUpdateMemberId(targetMemberId);
    try {
      await invoke('update_team_member_role', {
        teamCode: activeTeamCode,
        actorMemberId: activeTeamMemberId,
        targetMemberId,
        role,
      });
      const context = await invoke<TeamContext>('load_team_context', {
        teamCode: activeTeamCode,
        memberId: activeTeamMemberId,
      });
      setTeamSession(context.session);
      setTeamMembers(context.members);
      await refreshTeamActivity();
      showNotification({
        type: 'success',
        title: 'Role updated',
        message: 'Team member role changed.',
      });
    } catch (error) {
      console.error('Failed to update member role:', error);
      showNotification({
        type: 'error',
        title: 'Role update failed',
        message: 'Only owner can change member roles.',
      });
    } finally {
      setRoleUpdateMemberId(null);
    }
  };

  const handleAddTodo = (input: NewTodoInput) => {
    if (flowMode === 'team' && !teamSession) {
      showNotification({
        type: 'error',
        title: 'Team not ready',
        message: 'Join or create a team first.',
      });
      return false;
    }

    const title = input.title.trim();
    if (!title) {
      showNotification({
        type: 'error',
        title: 'Missing TODO title',
        message: 'Please enter a task title first.',
      });
      return false;
    }

    const dueAt = toIsoOrNull(input.dueAtLocal);
    const remindAt = toIsoOrNull(input.remindAtLocal);

    if (dueAt && remindAt && Date.parse(remindAt) > Date.parse(dueAt)) {
      showNotification({
        type: 'warning',
        title: 'Invalid times',
        message: 'Reminder time must be before the due time.',
      });
      return false;
    }

    const nextTodo: TodoItem = {
      id: createId(),
      title,
      description: input.description.trim(),
      createdAt: new Date().toISOString(),
      dueAt,
      remindAt,
      completed: false,
      reminderFiredAt: null,
      dueFiredAt: null,
      priority: input.priority,
      tags: input.tags,
      createdByMemberId: flowMode === 'team' ? teamSession?.memberId ?? null : null,
      createdByMemberName: flowMode === 'team' ? teamSession?.memberName ?? null : null,
      assigneeMemberId: flowMode === 'team' ? input.assigneeMemberId : null,
      assigneeMemberName:
        flowMode === 'team' && input.assigneeMemberId
          ? teamMemberNameById.get(input.assigneeMemberId) ?? null
          : null,
    };

    setTodos((current) => [nextTodo, ...current]);
    showNotification({
      type: 'success',
      title: 'TODO added',
      message: dueAt ? `Due: ${formatDateTime(dueAt)}` : 'Saved without a due date.',
    });
    return true;
  };

  const handleToggleTodo = (todoId: string) => {
    const target = todosRef.current.find((todo) => todo.id === todoId);
    if (!target) return;
    if (!canManageTodo(target)) {
      showNotification({
        type: 'warning',
        title: 'Permission denied',
        message: 'Your role cannot edit this task.',
      });
      return;
    }
    setTodos((current) => current.map((todo) => (todo.id === todoId ? { ...todo, completed: !todo.completed } : todo)));
  };

  const handleDeleteTodo = (todoId: string) => {
    const target = todosRef.current.find((todo) => todo.id === todoId);
    if (!target) return;
    if (!canManageTodo(target)) {
      showNotification({
        type: 'warning',
        title: 'Permission denied',
        message: 'Your role cannot delete this task.',
      });
      return;
    }
    setTodos((current) => current.filter((todo) => todo.id !== todoId));
  };

  const handleUpdateTodo = (updated: TodoItem) => {
    const previous = todosRef.current.find((todo) => todo.id === updated.id);
    if (previous && !canManageTodo(previous)) {
      showNotification({
        type: 'warning',
        title: 'Permission denied',
        message: 'Your role cannot edit this task.',
      });
      return;
    }
    setTodos((current) => current.map((todo) => (todo.id === updated.id ? updated : todo)));
    showNotification({ type: 'success', title: 'TODO updated', message: 'Changes saved.' });
  };

  const handleAddIdea = (input: NewIdeaInput) => {
    const content = input.content.trim();
    if (!content) {
      showNotification({
        type: 'error',
        title: 'Idea is empty',
        message: 'Add at least a short idea description.',
      });
      return false;
    }

    const nextIdea: IdeaItem = {
      id: createId(),
      title: input.title.trim(),
      content,
      createdAt: new Date().toISOString(),
      tags: input.tags,
    };

    setIdeas((current) => [nextIdea, ...current]);
    showNotification({
      type: 'success',
      title: 'Idea saved',
      message: 'Your idea is stored and ready for later.',
    });
    return true;
  };

  const handleDeleteIdea = (ideaId: string) => {
    setIdeas((current) => current.filter((idea) => idea.id !== ideaId));
  };

  const handleUpdateIdea = (updated: IdeaItem) => {
    setIdeas((current) => current.map((idea) => (idea.id === updated.id ? updated : idea)));
    showNotification({ type: 'success', title: 'Idea updated', message: 'Changes saved.' });
  };

  const handleConvertIdeaToTodo = (idea: IdeaItem) => {
    const nextTodo: TodoItem = {
      id: createId(),
      title: idea.title || 'Untitled idea',
      description: idea.content,
      createdAt: new Date().toISOString(),
      dueAt: null,
      remindAt: null,
      completed: false,
      reminderFiredAt: null,
      dueFiredAt: null,
      priority: 'medium',
      tags: [...idea.tags],
      createdByMemberId: flowMode === 'team' ? teamSession?.memberId ?? null : null,
      createdByMemberName: flowMode === 'team' ? teamSession?.memberName ?? null : null,
      assigneeMemberId: null,
      assigneeMemberName: null,
    };

    setTodos((current) => [nextTodo, ...current]);
    setIdeas((current) => current.filter((i) => i.id !== idea.id));
    showNotification({
      type: 'success',
      title: 'Converted to TODO',
      message: `"${nextTodo.title}" is now a task.`,
    });
    setCurrentView('todos');
  };

  const handleCreateTeam = async (displayName: string) => {
    if (teamAction) return;

    setTeamAction('create');
    try {
      const nextSession = await invoke<TeamSession>('create_team', {
        displayName: displayName.trim() || null,
      });

      setTeamSession(nextSession);
      setTeamMembers([]);
      setTeamActivity([]);
      setCurrentView('dashboard');
      showNotification({
        type: 'success',
        title: 'Team created',
        message: `Share this code: ${nextSession.teamCode}`,
        duration: 7000,
      });
    } catch (error) {
      console.error('Failed to create team:', error);
      showNotification({
        type: 'error',
        title: 'Create team failed',
        message: 'Could not create a new team in backend storage.',
      });
    } finally {
      setTeamAction(null);
    }
  };

  const handleJoinTeam = async (teamCode: string, displayName: string) => {
    if (teamAction) return;
    if (!teamCode.trim()) {
      showNotification({
        type: 'warning',
        title: 'Missing team code',
        message: 'Enter a valid team code first.',
      });
      return;
    }

    setTeamAction('join');
    try {
      const nextSession = await invoke<TeamSession>('join_team', {
        code: teamCode.trim(),
        displayName: displayName.trim() || null,
      });

      setTeamSession(nextSession);
      setTeamMembers([]);
      setTeamActivity([]);
      setCurrentView('dashboard');
      showNotification({
        type: 'success',
        title: 'Joined team',
        message: `Connected to ${nextSession.teamCode}.`,
      });
    } catch (error) {
      console.error('Failed to join team:', error);
      showNotification({
        type: 'error',
        title: 'Join failed',
        message: 'Team code not found or backend is unavailable.',
      });
    } finally {
      setTeamAction(null);
    }
  };

  const handleCopyTeamCode = async () => {
    if (!teamSession) return;

    try {
      await navigator.clipboard.writeText(teamSession.teamCode);
      showNotification({
        type: 'success',
        title: 'Code copied',
        message: `${teamSession.teamCode} copied to clipboard.`,
      });
    } catch {
      showNotification({
        type: 'warning',
        title: 'Copy failed',
        message: 'Clipboard is not available in this environment.',
      });
    }
  };

  const handleLeaveTeam = () => {
    setTeamSession(null);
    setTeamMembers([]);
    setTeamActivity([]);
    setLoadedStorageKey(null);
    setStorageReady(false);
    setTodos([]);
    setIdeas([]);
    todosRef.current = [];
    showNotification({
      type: 'info',
      title: 'Disconnected',
      message: 'You left Team Flow. Pick another team or switch to Personal Flow.',
    });
  };

  const renderView = () => {
    switch (currentView) {
      case 'dashboard':
        return <DashboardView todos={todos} ideas={ideas} />;
      case 'todos':
        return (
          <TodoView
            todos={todos}
            onAddTodo={handleAddTodo}
            onToggleComplete={handleToggleTodo}
            onDeleteTodo={handleDeleteTodo}
            onUpdateTodo={handleUpdateTodo}
            teamMode={flowMode === 'team'}
            teamMembers={teamMembers}
            currentMemberId={teamSession?.memberId ?? null}
            currentMemberRole={teamSession?.role ?? null}
            canManageTodo={canManageTodo}
          />
        );
      case 'ideas':
        return (
          <IdeasView
            ideas={ideas}
            onAddIdea={handleAddIdea}
            onDeleteIdea={handleDeleteIdea}
            onUpdateIdea={handleUpdateIdea}
            onConvertToTodo={handleConvertIdeaToTodo}
          />
        );
      case 'calendar':
        return <CalendarView todos={todos} />;
    }
  };

  return (
    <div className="app">
      <Titlebar />

      <div className="app-content">
        <Sidebar
          currentView={currentView}
          onViewChange={setCurrentView}
          collapsed={sidebarCollapsed}
          onToggleCollapse={() => setSidebarCollapsed((v) => !v)}
          appVersion={__APP_VERSION__}
        />

        <main className="main-view">
          <section className="flow-mode-panel">
            <div className="flow-mode-group">
              <button
                type="button"
                className={`flow-mode-btn ${flowMode === 'personal' ? 'active' : ''}`}
                onClick={() => setFlowMode('personal')}
              >
                <UserRound size={16} />
                <span>Personal Flow</span>
              </button>
              <button
                type="button"
                className={`flow-mode-btn ${flowMode === 'team' ? 'active' : ''}`}
                onClick={() => setFlowMode('team')}
              >
                <Users size={16} />
                <span>Team Flow</span>
              </button>
            </div>

            <span className="flow-mode-hint">
              {flowMode === 'personal'
                ? 'Personal mode keeps your tasks and ideas on this device.'
                : 'Team mode syncs your tasks and ideas through our backend server.'}
            </span>
          </section>

          {flowMode === 'team' && teamSession && (
            <section className="team-session-banner">
              <div className="team-session-main">
                <span className="team-session-label">Connected Team</span>
                <strong className="team-session-code">{teamSession.teamCode}</strong>
                <span className="team-session-meta">
                  {teamSession.memberName} ({teamRoleLabels[teamSession.role]}) - {teamSession.memberCount} members
                </span>
              </div>
              <div className="team-session-actions">
                <button type="button" className="btn btn-ghost" onClick={() => void handleCopyTeamCode()}>
                  <Copy size={15} />
                  Copy code
                </button>
                <button type="button" className="btn btn-danger" onClick={handleLeaveTeam}>
                  <LogOut size={15} />
                  Leave team
                </button>
              </div>
            </section>
          )}

          {flowMode === 'team' && teamSession && (
            <section className="team-panels-grid">
              <article className="panel team-members-panel">
                <div className="panel-header-row">
                  <h2 className="panel-title" style={{ marginBottom: 0 }}>Team Members</h2>
                  <span className="filter-count">{teamMembers.length} total</span>
                </div>

                <div className="team-members-list">
                  {teamMembers.map((member) => (
                    <div className="team-member-row" key={member.id}>
                      <div>
                        <h3 className="team-member-name">
                          {member.name} {member.id === teamSession.memberId ? '(You)' : ''}
                        </h3>
                        <p className="team-member-meta">Joined: {new Date(member.joinedAt).toLocaleString()}</p>
                      </div>
                      {teamSession.role === 'owner' && member.role !== 'owner' ? (
                        <div className="team-role-select">
                          <Select
                            value={member.role}
                            options={[
                              { value: 'member', label: 'Member' },
                              { value: 'admin', label: 'Admin' },
                            ]}
                            onChange={(value) => void handleUpdateMemberRole(member.id, value as TeamRole)}
                          />
                          {roleUpdateMemberId === member.id && (
                            <span className="filter-count">Saving...</span>
                          )}
                        </div>
                      ) : (
                        <span className={`team-role-pill role-${member.role}`}>
                          {teamRoleLabels[member.role]}
                        </span>
                      )}
                    </div>
                  ))}
                </div>
              </article>

              {teamSession.owner && (
                <article className="panel team-activity-panel">
                  <div className="panel-header-row">
                    <h2 className="panel-title" style={{ marginBottom: 0 }}>Activity Feed (Owner)</h2>
                    <button
                      type="button"
                      className="btn btn-ghost"
                      onClick={() => void refreshTeamActivity()}
                      disabled={refreshingActivity}
                    >
                      {refreshingActivity ? 'Refreshing...' : 'Refresh'}
                    </button>
                  </div>

                  {teamActivity.length === 0 && (
                    <div className="empty-state">
                      <p>No activity yet.</p>
                    </div>
                  )}

                  <div className="team-activity-list">
                    {teamActivity.map((item) => (
                      <div className="team-activity-item" key={item.id}>
                        <div className="team-activity-top">
                          <strong>{item.actorMemberName}</strong>
                          <span>{new Date(item.createdAt).toLocaleString()}</span>
                        </div>
                        <p>{item.details}</p>
                      </div>
                    ))}
                  </div>
                </article>
              )}
            </section>
          )}

          {flowMode === 'team' && !teamSession ? (
            <TeamGateView
              onCreateTeam={handleCreateTeam}
              onJoinTeam={handleJoinTeam}
              creating={teamAction === 'create'}
              joining={teamAction === 'join'}
            />
          ) : (
            renderView()
          )}
        </main>
      </div>

      <NotificationContainer />
    </div>
  );
};

function App() {
  return (
    <NotificationProvider>
      <AppContent />
    </NotificationProvider>
  );
}

export default App;

