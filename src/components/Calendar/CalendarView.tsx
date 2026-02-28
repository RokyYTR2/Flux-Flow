import { useMemo, useState } from 'react';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import type { TodoItem } from '../../types/todo';

interface CalendarViewProps {
  todos: TodoItem[];
}

type CalendarEventType = 'due' | 'reminder';

interface CalendarEvent {
  key: string;
  title: string;
  type: CalendarEventType;
  time: number;
  completed: boolean;
}

interface CalendarCell {
  date: Date | null;
  events: CalendarEvent[];
}

const weekdayLabels = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

const toDateOrNull = (iso: string | null) => {
  if (!iso) return null;
  const ms = Date.parse(iso);
  if (Number.isNaN(ms)) return null;
  return new Date(ms);
};

const dateKey = (d: Date) => `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;

const CalendarView = ({ todos }: CalendarViewProps) => {
  const now = new Date();
  const [monthStart, setMonthStart] = useState(() => new Date(now.getFullYear(), now.getMonth(), 1));
  const [monthTransition, setMonthTransition] = useState<'next' | 'prev' | 'jump' | null>(null);

  const { cells, monthLabel, dueCount, reminderCount } = useMemo(() => {
    const eventMap = new Map<string, CalendarEvent[]>();

    for (const todo of todos) {
      const dueDate = toDateOrNull(todo.dueAt);
      if (dueDate) {
        const key = dateKey(dueDate);
        const events = eventMap.get(key) ?? [];
        events.push({
          key: `${todo.id}-due-${todo.dueAt}`,
          title: todo.title,
          type: 'due',
          time: dueDate.getTime(),
          completed: todo.completed,
        });
        eventMap.set(key, events);
      }

      const remindDate = toDateOrNull(todo.remindAt);
      if (remindDate) {
        const key = dateKey(remindDate);
        const events = eventMap.get(key) ?? [];
        events.push({
          key: `${todo.id}-reminder-${todo.remindAt}`,
          title: todo.title,
          type: 'reminder',
          time: remindDate.getTime(),
          completed: todo.completed,
        });
        eventMap.set(key, events);
      }
    }

    for (const events of eventMap.values()) {
      events.sort((a, b) => a.time - b.time);
    }

    const firstDay = new Date(monthStart.getFullYear(), monthStart.getMonth(), 1);
    const daysInMonth = new Date(firstDay.getFullYear(), firstDay.getMonth() + 1, 0).getDate();
    const lead = firstDay.getDay();
    const cellCount = 42;

    const nextCells: CalendarCell[] = Array.from({ length: cellCount }, (_, index) => {
      const dayNumber = index - lead + 1;

      if (dayNumber < 1 || dayNumber > daysInMonth) {
        return { date: null, events: [] };
      }

      const date = new Date(firstDay.getFullYear(), firstDay.getMonth(), dayNumber);
      const events = eventMap.get(dateKey(date)) ?? [];
      return { date, events };
    });

    const inMonthEvents = nextCells.flatMap((cell) => cell.events);
    const due = inMonthEvents.filter((event) => event.type === 'due').length;
    const reminder = inMonthEvents.filter((event) => event.type === 'reminder').length;

    return {
      cells: nextCells,
      monthLabel: firstDay.toLocaleString('en-US', { month: 'long', year: 'numeric' }),
      dueCount: due,
      reminderCount: reminder,
    };
  }, [monthStart, todos]);

  const todayKey = dateKey(new Date());
  const monthKey = `${monthStart.getFullYear()}-${monthStart.getMonth()}`;

  const goToPreviousMonth = () => {
    setMonthTransition('prev');
    setMonthStart((current) => new Date(current.getFullYear(), current.getMonth() - 1, 1));
  };

  const goToNextMonth = () => {
    setMonthTransition('next');
    setMonthStart((current) => new Date(current.getFullYear(), current.getMonth() + 1, 1));
  };

  const jumpToCurrentMonth = () => {
    const nowDate = new Date();
    setMonthTransition('jump');
    setMonthStart(new Date(nowDate.getFullYear(), nowDate.getMonth(), 1));
  };

  return (
    <section className="stack">
      <header className="section-header">
        <div className="section-badge">CALENDAR</div>
        <h1 className="section-title">Calendar</h1>
        <p className="section-subtitle">Monthly overview of reminder and due dates for your tasks.</p>
      </header>

      <article className="panel">
        <div className="calendar-toolbar">
          <button type="button" className="btn btn-ghost" onClick={goToPreviousMonth} aria-label="Previous month">
            <ChevronLeft size={15} />
          </button>
          <div className="calendar-toolbar-center">
            <h2 className="panel-title calendar-title">{monthLabel}</h2>
            <p className="calendar-subtitle">Due: {dueCount} | Reminders: {reminderCount}</p>
          </div>
          <button type="button" className="btn btn-ghost" onClick={goToNextMonth} aria-label="Next month">
            <ChevronRight size={15} />
          </button>
          <button type="button" className="btn btn-ghost" onClick={jumpToCurrentMonth}>Today</button>
        </div>

        <div
          key={monthKey}
          className={`calendar-month-frame ${
            monthTransition === 'next'
              ? 'calendar-month-next'
              : monthTransition === 'prev'
                ? 'calendar-month-prev'
                : monthTransition === 'jump'
                  ? 'calendar-month-jump'
                  : ''
          }`}
        >
          <div className="calendar-scroll">
          <div className="calendar-weekdays">
            {weekdayLabels.map((label) => (
              <div key={label} className="calendar-weekday">{label}</div>
            ))}
          </div>

          <div className="calendar-grid">
            {cells.map((cell, index) => {
              if (!cell.date) {
                return <div key={`empty-${index}`} className="calendar-day calendar-day-empty" />;
              }

              const currentKey = dateKey(cell.date);
              const isToday = currentKey === todayKey;

              return (
                <div key={currentKey} className={`calendar-day ${isToday ? 'calendar-day-today' : ''}`}>
                  <div className="calendar-day-head">
                    <span className="calendar-day-number">{cell.date.getDate()}</span>
                  </div>
                  <div className="calendar-event-list">
                    {cell.events.length === 0 ? (
                      <span className="calendar-empty-label">No events</span>
                    ) : (
                      <>
                        {cell.events.slice(0, 3).map((event) => (
                          <div
                            key={event.key}
                            className={`calendar-event calendar-event-${event.type} ${event.completed ? 'calendar-event-done' : ''}`}
                            title={event.title}
                          >
                            <span className="calendar-event-type">{event.type === 'due' ? 'Due' : 'Remind'}</span>
                            <span className="calendar-event-title">{event.title}</span>
                          </div>
                        ))}
                        {cell.events.length > 3 && (
                          <span className="calendar-more-events">+{cell.events.length - 3} more</span>
                        )}
                      </>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
          </div>
        </div>
      </article>
    </section>
  );
};

export default CalendarView;
