export interface IdeaItem {
  id: string;
  title: string;
  content: string;
  createdAt: string;
  tags: string[];
}

export interface NewIdeaInput {
  title: string;
  content: string;
  tags: string[];
}
