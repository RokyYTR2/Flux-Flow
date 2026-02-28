export type TeamRole = 'owner' | 'admin' | 'member';

export interface TeamSession {
  teamCode: string;
  memberId: string;
  memberName: string;
  authToken: string;
  role: TeamRole;
  owner: boolean;
  memberCount: number;
}

export interface TeamMember {
  id: string;
  name: string;
  role: TeamRole;
  joinedAt: string;
}

export interface TeamActivityItem {
  id: string;
  createdAt: string;
  actorMemberId: string;
  actorMemberName: string;
  action: string;
  details: string;
}

export interface TeamContext {
  session: TeamSession;
  members: TeamMember[];
}
