import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type Board = {
  id: string;
  title: string;
  columns: string[];
};

type Task = {
  id: string;
  title: string;
  board: string;
  column: string;
  tags: string[];
  due?: string | null;
  created?: string | null;
  updated?: string | null;
  body: string;
};

type BoardColumn = {
  name: string;
  tasks: Task[];
};

type BoardWithTasks = {
  board: Board;
  columns: BoardColumn[];
};

type VaultInfo = { path: string };

export default function App() {
  const [vaultInfo, setVaultInfo] = useState<VaultInfo | null>(null);
  const [boards, setBoards] = useState<Board[]>([]);
  const [activeBoardId, setActiveBoardId] = useState<string | null>(null);
  const [boardData, setBoardData] = useState<BoardWithTasks | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState<boolean>(true);

  const activeBoard = useMemo(
    () => boards.find((b) => b.id === activeBoardId) ?? null,
    [boards, activeBoardId]
  );

  useEffect(() => {
    (async () => {
      try {
        setLoading(true);
        setError(null);
        const info = await invoke<VaultInfo>("vault_info");
        setVaultInfo(info);

        const bs = await invoke<Board[]>("list_boards");
        setBoards(bs);
        setActiveBoardId((prev) => prev ?? (bs[0]?.id ?? null));
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  useEffect(() => {
    if (!activeBoardId) return;
    (async () => {
      try {
        setError(null);
        const data = await invoke<BoardWithTasks>("get_board_with_tasks", {
          boardId: activeBoardId,
        });
        setBoardData(data);
      } catch (e) {
        setError(String(e));
      }
    })();
  }, [activeBoardId]);

  return (
    <main className="app">
      <header className="topbar">
        <div className="title">
          <h1>Kanban Vault</h1>
          <div className="subtitle">
            macOS-only • Tauri 2 • Markdown vault
          </div>
        </div>

        <div className="controls">
          <label className="label">
            Board
            <select
              value={activeBoardId ?? ""}
              onChange={(e) => setActiveBoardId(e.currentTarget.value)}
              disabled={boards.length === 0}
            >
              {boards.map((b) => (
                <option key={b.id} value={b.id}>
                  {b.title}
                </option>
              ))}
            </select>
          </label>
        </div>
      </header>

      <section className="meta">
        {vaultInfo ? (
          <div className="metaRow">
            <span className="metaLabel">Vault:</span>
            <code className="metaValue">{vaultInfo.path}</code>
          </div>
        ) : null}

        {activeBoard ? (
          <div className="metaRow">
            <span className="metaLabel">Board ID:</span>
            <code className="metaValue">{activeBoard.id}</code>
          </div>
        ) : null}
      </section>

      {loading ? <div className="state">Loading…</div> : null}
      {error ? (
        <div className="state error">
          <div className="stateTitle">Error</div>
          <pre className="stateBody">{error}</pre>
        </div>
      ) : null}

      {boardData ? (
        <section className="board">
          {boardData.columns.map((col) => (
            <div key={col.name} className="column">
              <div className="columnHeader">
                <div className="columnName">{col.name}</div>
                <div className="columnCount">{col.tasks.length}</div>
              </div>

              <div className="cards">
                {col.tasks.map((t) => (
                  <article key={t.id} className="card">
                    <div className="cardTitle">{t.title}</div>
                    <div className="cardMeta">
                      <code>{t.id}</code>
                      {t.tags?.length ? (
                        <span className="tags">
                          {t.tags.map((tag) => (
                            <span key={tag} className="tag">
                              {tag}
                            </span>
                          ))}
                        </span>
                      ) : null}
                    </div>
                    {t.body ? <div className="cardBody">{t.body}</div> : null}
                  </article>
                ))}
              </div>
            </div>
          ))}
        </section>
      ) : !loading ? (
        <div className="state">No board loaded.</div>
      ) : null}
    </main>
  );
}
