import { useMemo, useState } from 'react';
import { ArrowRightLeft, Lightbulb, Pencil, Save, Trash2, X } from 'lucide-react';
import type { IdeaItem, NewIdeaInput } from '../../types/idea';
import TagInput from '../TagInput';

interface IdeasViewProps {
  ideas: IdeaItem[];
  onAddIdea: (input: NewIdeaInput) => boolean;
  onDeleteIdea: (ideaId: string) => void;
  onUpdateIdea: (idea: IdeaItem) => void;
  onConvertToTodo: (idea: IdeaItem) => void;
}

const IdeasView = ({ ideas, onAddIdea, onDeleteIdea, onUpdateIdea, onConvertToTodo }: IdeasViewProps) => {
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [tags, setTags] = useState<string[]>([]);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editState, setEditState] = useState<Partial<IdeaItem>>({});

  const sortedIdeas = useMemo(() => {
    return [...ideas].sort((left, right) => Date.parse(right.createdAt) - Date.parse(left.createdAt));
  }, [ideas]);

  const handleSubmit = (event: React.FormEvent) => {
    event.preventDefault();

    const created = onAddIdea({ title, content, tags });
    if (!created) return;

    setTitle('');
    setContent('');
    setTags([]);
  };

  const startEdit = (idea: IdeaItem) => {
    setEditingId(idea.id);
    setEditState({ title: idea.title, content: idea.content, tags: [...idea.tags] });
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditState({});
  };

  const saveEdit = (idea: IdeaItem) => {
    onUpdateIdea({
      ...idea,
      title: editState.title?.trim() ?? idea.title,
      content: editState.content?.trim() || idea.content,
      tags: editState.tags || idea.tags,
    });
    setEditingId(null);
    setEditState({});
  };

  return (
    <section className="stack">
      <header className="section-header">
        <div className="section-badge">IDEA LAB</div>
        <h1 className="section-title">Ideas</h1>
        <p className="section-subtitle">
          Save every idea quickly so you do not lose it. Everything is stored locally in the app.
        </p>
      </header>

      <article className="panel">
        <h2 className="panel-title">New idea</h2>
        <form className="form-grid" onSubmit={handleSubmit}>
          <label className="field">
            <span>Title (optional)</span>
            <input
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              placeholder="e.g. concept for a new feature"
              maxLength={140}
            />
          </label>

          <label className="field">
            <span>Idea details</span>
            <textarea
              value={content}
              onChange={(event) => setContent(event.target.value)}
              placeholder="Write your idea..."
              rows={5}
              maxLength={1400}
              required
            />
          </label>

          <div className="field">
            <span>Tags</span>
            <TagInput tags={tags} onChange={setTags} />
          </div>

          <button type="submit" className="btn btn-primary">
            Save idea
          </button>
        </form>
      </article>

      <article className="panel">
        <h2 className="panel-title">Saved ideas</h2>
        {sortedIdeas.length === 0 && (
          <div className="empty-state">
            <Lightbulb size={20} />
            <p>No ideas saved yet.</p>
          </div>
        )}

        <div className="ideas-list">
          {sortedIdeas.map((idea) => {
            const isEditing = editingId === idea.id;

            return (
              <div key={idea.id} className="idea-card">
                {isEditing ? (
                  <div className="form-grid">
                    <label className="field">
                      <span>Title</span>
                      <input
                        value={editState.title ?? ''}
                        onChange={(e) => setEditState((s) => ({ ...s, title: e.target.value }))}
                        maxLength={140}
                      />
                    </label>
                    <label className="field">
                      <span>Content</span>
                      <textarea
                        value={editState.content ?? ''}
                        onChange={(e) => setEditState((s) => ({ ...s, content: e.target.value }))}
                        rows={4}
                        maxLength={1400}
                      />
                    </label>
                    <div className="field">
                      <span>Tags</span>
                      <TagInput
                        tags={editState.tags || []}
                        onChange={(t) => setEditState((s) => ({ ...s, tags: t }))}
                      />
                    </div>
                    <div className="todo-actions">
                      <button type="button" className="btn btn-primary" onClick={() => saveEdit(idea)}>
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
                    <div className="idea-top">
                      <h3 className="idea-title">{idea.title || 'Untitled'}</h3>
                      <div className="idea-actions">
                        <button type="button" className="btn btn-ghost" onClick={() => onConvertToTodo(idea)}>
                          <ArrowRightLeft size={15} />
                          To TODO
                        </button>
                        <button type="button" className="btn btn-ghost" onClick={() => startEdit(idea)}>
                          <Pencil size={15} />
                          Edit
                        </button>
                        <button type="button" className="btn btn-danger" onClick={() => {
                          if (window.confirm(`Delete "${idea.title || 'Untitled'}"?`)) onDeleteIdea(idea.id);
                        }}>
                          <Trash2 size={15} />
                          Delete
                        </button>
                      </div>
                    </div>
                    <p className="idea-content">{idea.content}</p>
                    {idea.tags.length > 0 && (
                      <div className="todo-tags">
                        {idea.tags.map((tag) => (
                          <span key={tag} className="tag tag-display">{tag}</span>
                        ))}
                      </div>
                    )}
                    <p className="idea-footer">
                      Saved {new Date(idea.createdAt).toLocaleString('en-US', { dateStyle: 'medium', timeStyle: 'short' })}
                    </p>
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

export default IdeasView;
