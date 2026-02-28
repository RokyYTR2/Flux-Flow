import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from 'react';

export type NotificationType = 'success' | 'error' | 'info' | 'warning';

export interface AppNotification {
  id: string;
  type: NotificationType;
  title: string;
  message?: string;
  duration?: number;
  expiring?: boolean;
}

interface NotificationContextType {
  notifications: AppNotification[];
  showNotification: (notification: Omit<AppNotification, 'id' | 'expiring'>) => void;
  removeNotification: (id: string) => void;
  markExpiring: (id: string) => void;
}

const NotificationContext = createContext<NotificationContextType | undefined>(undefined);

const createId = () => {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID();
  }

  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
};

export const NotificationProvider = ({ children }: { children: ReactNode }) => {
  const [notifications, setNotifications] = useState<AppNotification[]>([]);

  const removeNotification = useCallback((id: string) => {
    setNotifications((prev) => prev.filter((notification) => notification.id !== id));
  }, []);

  const markExpiring = useCallback(
    (id: string) => {
      setNotifications((prev) =>
        prev.map((notification) =>
          notification.id === id ? { ...notification, expiring: true } : notification
        )
      );

      window.setTimeout(() => {
        removeNotification(id);
      }, 220);
    },
    [removeNotification]
  );

  const showNotification = useCallback(
    (notification: Omit<AppNotification, 'id' | 'expiring'>) => {
      const id = createId();
      const newNotification: AppNotification = {
        ...notification,
        id,
        expiring: false,
      };

      setNotifications((prev) => [...prev, newNotification]);

      const duration = notification.duration ?? 3200;
      if (duration > 0) {
        window.setTimeout(() => {
          markExpiring(id);
        }, duration);
      }
    },
    [markExpiring]
  );

  const value = useMemo(
    () => ({
      notifications,
      showNotification,
      removeNotification,
      markExpiring,
    }),
    [notifications, showNotification, removeNotification, markExpiring]
  );

  return <NotificationContext.Provider value={value}>{children}</NotificationContext.Provider>;
};

export const useNotification = () => {
  const context = useContext(NotificationContext);

  if (!context) {
    throw new Error('useNotification must be used within NotificationProvider');
  }

  return context;
};
