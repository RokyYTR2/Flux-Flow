export type Priority = 'low' | 'medium' | 'high';

export interface TodoItem {
  id: string;
  title: string;
  description: string;
  createdAt: string;
  dueAt: string | null;
  remindAt: string | null;
  completed: boolean;
  reminderFiredAt: string | null;
  dueFiredAt: string | null;
  priority: Priority;
  tags: string[];
  createdByMemberId: string | null;
  createdByMemberName: string | null;
  assigneeMemberId: string | null;
  assigneeMemberName: string | null;
}

export interface NewTodoInput {
  title: string;
  description: string;
  dueAtLocal: string;
  remindAtLocal: string;
  priority: Priority;
  tags: string[];
  assigneeMemberId: string | null;
}
