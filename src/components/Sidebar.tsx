import { BarChart3, CalendarDays, CheckCheck, Lightbulb, PanelLeftClose, PanelLeftOpen } from 'lucide-react';
import type { AppView } from '../types/view';

interface SidebarProps {
  currentView: AppView;
  onViewChange: (view: AppView) => void;
  collapsed: boolean;
  onToggleCollapse: () => void;
  appVersion: string;
}

const menuItems = [
  { id: 'dashboard' as const, label: 'Dashboard', icon: BarChart3 },
  { id: 'todos' as const, label: 'Todo Planner', icon: CheckCheck },
  { id: 'ideas' as const, label: 'Ideas', icon: Lightbulb },
  { id: 'calendar' as const, label: 'Calendar', icon: CalendarDays },
];

const Sidebar = ({ currentView, onViewChange, collapsed, onToggleCollapse, appVersion }: SidebarProps) => {
  return (
    <aside className={`sidebar ${collapsed ? 'collapsed' : ''}`}>
      <div className="sidebar-head">
        <div className="sidebar-logo-mark">F</div>
        {!collapsed && <span className="sidebar-logo-text">Flux Flow</span>}
      </div>

      <div className="sidebar-stack">
        {menuItems.map((item) => {
          const Icon = item.icon;
          const active = currentView === item.id;

          return (
            <button
              key={item.id}
              type="button"
              className={`sidebar-item ${active ? 'active' : ''}`}
              onClick={() => onViewChange(item.id)}
              title={collapsed ? item.label : undefined}
            >
              <Icon size={20} />
              {!collapsed && <span>{item.label}</span>}
            </button>
          );
        })}
      </div>
      <div className="sidebar-bottom">
        <span className="sidebar-version" title={`Version ${appVersion}`}>v{appVersion}</span>
        <button type="button" className="sidebar-collapse-btn" onClick={onToggleCollapse} title={collapsed ? 'Expand' : 'Collapse'}>
          {collapsed ? <PanelLeftOpen size={16} /> : <PanelLeftClose size={16} />}
        </button>
      </div>
    </aside>
  );
};

export default Sidebar;
