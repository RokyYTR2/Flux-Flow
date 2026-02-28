import { AlertTriangle, CheckCircle2, Info, X, XCircle } from 'lucide-react';
import { useNotification, type NotificationType } from '../contexts/NotificationContext';
import './NotificationContainer.css';

const iconByType = (type: NotificationType) => {
  switch (type) {
    case 'success':
      return <CheckCircle2 size={20} />;
    case 'error':
      return <XCircle size={20} />;
    case 'warning':
      return <AlertTriangle size={20} />;
    case 'info':
    default:
      return <Info size={20} />;
  }
};

const NotificationContainer = () => {
  const { notifications, markExpiring } = useNotification();

  return (
    <div className="notification-container">
      {notifications.map((notification) => (
        <div
          key={notification.id}
          className={`notification notification-${notification.type} ${
            notification.expiring ? 'notification-removing' : ''
          }`}
          onClick={() => markExpiring(notification.id)}
        >
          <div className="notification-icon">{iconByType(notification.type)}</div>
          <div className="notification-content">
            <div className="notification-title">{notification.title}</div>
            {notification.message && <div className="notification-message">{notification.message}</div>}
          </div>
          <button
            className="notification-close"
            onClick={(event) => {
              event.stopPropagation();
              markExpiring(notification.id);
            }}
          >
            <X size={16} />
          </button>
        </div>
      ))}
    </div>
  );
};

export default NotificationContainer;
