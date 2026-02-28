import { LogIn, PlusCircle, Users } from 'lucide-react';
import { FormEvent, useMemo, useState } from 'react';

interface TeamGateViewProps {
  onCreateTeam: (displayName: string) => Promise<void>;
  onJoinTeam: (teamCode: string, displayName: string) => Promise<void>;
  creating: boolean;
  joining: boolean;
}

const formatTeamCode = (value: string) => {
  const cleaned = value
    .toUpperCase()
    .replace(/[^A-Z0-9]/g, '')
    .slice(0, 8);

  if (cleaned.length <= 4) return cleaned;
  return `${cleaned.slice(0, 4)}-${cleaned.slice(4)}`;
};

const TeamGateView = ({ onCreateTeam, onJoinTeam, creating, joining }: TeamGateViewProps) => {
  const [createName, setCreateName] = useState('');
  const [joinCode, setJoinCode] = useState('');
  const [joinName, setJoinName] = useState('');
  const normalizedJoinCode = useMemo(() => formatTeamCode(joinCode), [joinCode]);

  const handleCreateTeam = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    void onCreateTeam(createName);
  };

  const handleJoinTeam = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    void onJoinTeam(normalizedJoinCode, joinName);
  };

  const canJoin = normalizedJoinCode.length === 9 && !joining;

  return (
    <section className="stack">
      <header className="section-header">
        <span className="section-badge">Team Flow</span>
        <h1 className="section-title">Create a Team or Join with Code</h1>
        <p className="section-subtitle">
          Team data is stored in the Rust backend. Share the generated team code with others so they can join the same
          workspace.
        </p>
      </header>

      <div className="team-gate-grid">
        <form className="panel form-grid" onSubmit={handleCreateTeam}>
          <h2 className="panel-title">Create Team</h2>
          <label className="field">
            <span>Your Name</span>
            <input
              type="text"
              value={createName}
              onChange={(event) => setCreateName(event.target.value)}
              placeholder="Owner name (optional)"
              maxLength={40}
            />
          </label>

          <button type="submit" className="btn btn-primary" disabled={creating}>
            <PlusCircle size={16} />
            {creating ? 'Creating...' : 'Create Team'}
          </button>
        </form>

        <form className="panel form-grid" onSubmit={handleJoinTeam}>
          <h2 className="panel-title">Join Team</h2>
          <label className="field">
            <span>Team Code</span>
            <input
              type="text"
              value={normalizedJoinCode}
              onChange={(event) => setJoinCode(event.target.value)}
              placeholder="ABCD-1234"
              spellCheck={false}
              autoCapitalize="characters"
              maxLength={9}
            />
          </label>
          <label className="field">
            <span>Your Name</span>
            <input
              type="text"
              value={joinName}
              onChange={(event) => setJoinName(event.target.value)}
              placeholder="Member name (optional)"
              maxLength={40}
            />
          </label>

          <button type="submit" className="btn btn-primary" disabled={!canJoin}>
            <LogIn size={16} />
            {joining ? 'Joining...' : 'Join Team'}
          </button>
        </form>
      </div>

      <div className="team-gate-hint">
        <Users size={15} />
        <span>
          Code format is <strong>XXXX-XXXX</strong>. Team owner receives the code after creating a team.
        </span>
      </div>
    </section>
  );
};

export default TeamGateView;
