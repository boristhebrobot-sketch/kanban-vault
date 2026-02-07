import { useEffect, useMemo, useState } from "react";
import type { DragEvent } from "react";
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

type Project = {
  id: string;
  title: string;
  owner?: string | null;
  description?: string | null;
};

type Epic = {
  id: string;
  title: string;
  project_id?: string | null;
  owner?: string | null;
  description?: string | null;
};

type WizardType = "project" | "epic" | "story";

type WizardErrors = Record<string, string>;

type AutoFillResponse = {
  title?: string | null;
  asA?: string | null;
  iWant?: string | null;
  soThat?: string | null;
  acceptanceCriteria?: string[] | null;
};

export default function App() {
  const [vaultInfo, setVaultInfo] = useState<VaultInfo | null>(null);
  const [boards, setBoards] = useState<Board[]>([]);
  const [activeBoardId, setActiveBoardId] = useState<string | null>(null);
  const [boardData, setBoardData] = useState<BoardWithTasks | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState<boolean>(true);

  const [isWizardOpen, setIsWizardOpen] = useState(false);
  const [wizardStep, setWizardStep] = useState(0);
  const [wizardType, setWizardType] = useState<WizardType>("story");
  const [wizardErrors, setWizardErrors] = useState<WizardErrors>({});
  const [projects, setProjects] = useState<Project[]>([]);
  const [epics, setEpics] = useState<Epic[]>([]);

  const [title, setTitle] = useState("");
  const [owner, setOwner] = useState("");
  const [description, setDescription] = useState("");
  const [projectId, setProjectId] = useState("");
  const [epicId, setEpicId] = useState("");
  const [asA, setAsA] = useState("");
  const [iWant, setIWant] = useState("");
  const [soThat, setSoThat] = useState("");
  const [acceptanceCriteria, setAcceptanceCriteria] = useState<string[]>([""]);
  const [useAiAutofill, setUseAiAutofill] = useState(true);
  const [aiError, setAiError] = useState<string | null>(null);
  const [isAutofilling, setIsAutofilling] = useState(false);


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

  const refreshBoard = async () => {
    if (!activeBoardId) return;
    const data = await invoke<BoardWithTasks>("get_board_with_tasks", {
      boardId: activeBoardId,
    });
    setBoardData(data);
  };

  const openWizard = async () => {
    setWizardErrors({});
    setWizardStep(0);
    setIsWizardOpen(true);
    setAiError(null);
    const [proj, epi] = await Promise.all([
      invoke<Project[]>("list_projects"),
      invoke<Epic[]>("list_epics"),
    ]);
    setProjects(proj);
    setEpics(epi);
  };

  const closeWizard = () => {
    setIsWizardOpen(false);
    setWizardErrors({});
    setAiError(null);
  };

  const resetWizardForm = () => {
    setTitle("");
    setOwner("");
    setDescription("");
    setProjectId("");
    setEpicId("");
    setAsA("");
    setIWant("");
    setSoThat("");
    setAcceptanceCriteria([""]);
  };

  const validateWizard = (): boolean => {
    const errors: WizardErrors = {};
    if (!title.trim()) errors.title = "Title is required.";
    if (wizardType === "epic" && !projectId) {
      errors.projectId = "Select a project to link this epic.";
    }
    if (wizardType === "story") {
      if (!description.trim()) errors.description = "Description is required.";
      if (!asA.trim()) errors.asA = "As a… is required.";
      if (!iWant.trim()) errors.iWant = "I want… is required.";
      if (!soThat.trim()) errors.soThat = "So that… is required.";
    }
    setWizardErrors(errors);
    return Object.keys(errors).length === 0;
  };

  const handleSubmitWizard = async () => {
    if (!validateWizard()) return;

    if (wizardType === "project") {
      await invoke("create_project", {
        payload: {
          title,
          owner: owner || null,
          description: description || null,
        },
      });
    } else if (wizardType === "epic") {
      await invoke("create_epic", {
        payload: {
          title,
          projectId: projectId || null,
          owner: owner || null,
          description: description || null,
        },
      });
    } else {
      await invoke("create_story", {
        payload: {
          title,
          projectId: projectId || null,
          epicId: epicId || null,
          owner: owner || null,
          description: description || null,
          asA: asA || null,
          iWant: iWant || null,
          soThat: soThat || null,
          acceptanceCriteria: acceptanceCriteria.filter((c) => c.trim().length),
          column: "Backlog",
        },
      });
      await refreshBoard();
    }

    resetWizardForm();
    closeWizard();
  };

  const handleAutoFill = async () => {
    if (!useAiAutofill) return;
    setAiError(null);
    if (!description.trim()) {
      setAiError("Add a short description before running auto-fill.");
      return;
    }
    try {
      setIsAutofilling(true);
      const result = await invoke<AutoFillResponse>("openai_autofill_story", {
        payload: {
          title: title || null,
          description: description.trim(),
          asA: asA || null,
          iWant: iWant || null,
          soThat: soThat || null,
          acceptanceCriteria: acceptanceCriteria.filter((c) => c.trim().length),
        },
      });

      if (!title && result.title) setTitle(result.title);
      if (!asA && result.asA) setAsA(result.asA);
      if (!iWant && result.iWant) setIWant(result.iWant);
      if (!soThat && result.soThat) setSoThat(result.soThat);
      if (
        result.acceptanceCriteria?.length &&
        acceptanceCriteria.every((c) => !c.trim())
      ) {
        setAcceptanceCriteria(result.acceptanceCriteria);
      }
    } catch (e) {
      setAiError(String(e));
    } finally {
      setIsAutofilling(false);
    }
  };

  const handleDragStart = (taskId: string, column: string) =>
    (event: DragEvent<HTMLElement>) => {
      event.dataTransfer.setData("text/plain", taskId);
      event.dataTransfer.setData("text/column", column);
      event.dataTransfer.effectAllowed = "move";
    };

  const handleDrop = (column: string) => async (
    event: DragEvent<HTMLElement>
  ) => {
    event.preventDefault();
    const taskId = event.dataTransfer.getData("text/plain");
    const fromColumn = event.dataTransfer.getData("text/column");
    if (!taskId || fromColumn === column) return;
    await invoke("update_task_column", {
      payload: { taskId, column },
    });
    await refreshBoard();
  };

  const handleDragOver = (event: DragEvent<HTMLElement>) => {
    event.preventDefault();
  };

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
          <button className="button" onClick={openWizard}>
            New item
          </button>
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
            <div
              key={col.name}
              className="column"
              onDrop={handleDrop(col.name)}
              onDragOver={handleDragOver}
            >
              <div className="columnHeader">
                <div className="columnName">{col.name}</div>
                <div className="columnCount">{col.tasks.length}</div>
              </div>

              <div className="cards">
                {col.tasks.map((t) => (
                  <article
                    key={t.id}
                    className="card"
                    draggable
                    onDragStart={handleDragStart(t.id, col.name)}
                  >
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

      {isWizardOpen ? (
        <div className="modalBackdrop" onClick={closeWizard}>
          <div
            className="modal"
            onClick={(event) => event.stopPropagation()}
          >
            <header className="modalHeader">
              <div>
                <div className="modalTitle">Create new item</div>
                <div className="modalSubtitle">
                  Guided setup for projects, epics, and stories
                </div>
              </div>
              <button className="ghostButton" onClick={closeWizard}>
                Close
              </button>
            </header>

            <div className="wizardSteps">
              {[
                "Type",
                "Details",
                wizardType === "story" ? "Story" : "Review",
              ].map((step, index) => (
                <button
                  key={step}
                  className={`step ${wizardStep === index ? "active" : ""}`}
                  onClick={() => setWizardStep(index)}
                >
                  {step}
                </button>
              ))}
            </div>

            {wizardStep === 0 ? (
              <div className="wizardBody">
                <div className="field">
                  <label>Item type</label>
                  <div className="segmented">
                    {(["project", "epic", "story"] as WizardType[]).map((t) => (
                      <button
                        key={t}
                        className={`segment ${wizardType === t ? "active" : ""}`}
                        onClick={() => setWizardType(t)}
                      >
                        {t}
                      </button>
                    ))}
                  </div>
                </div>
                <p className="hint">
                  Projects group epics, epics group stories, stories appear on the
                  board.
                </p>
              </div>
            ) : null}

            {wizardStep === 1 ? (
              <div className="wizardBody">
                <div className="field">
                  <label>Title *</label>
                  <input
                    value={title}
                    onChange={(e) => setTitle(e.currentTarget.value)}
                    placeholder="Short, descriptive title"
                  />
                  {wizardErrors.title ? (
                    <div className="errorText">{wizardErrors.title}</div>
                  ) : null}
                </div>

                <div className="field">
                  <label>Owner</label>
                  <input
                    value={owner}
                    onChange={(e) => setOwner(e.currentTarget.value)}
                    placeholder="Dylan, Boris, or a team"
                  />
                </div>

                <div className="field">
                  <label>Description {wizardType === "story" ? "*" : ""}</label>
                  <textarea
                    value={description}
                    onChange={(e) => setDescription(e.currentTarget.value)}
                    placeholder="Describe the project/epic/story"
                  />
                  {wizardErrors.description ? (
                    <div className="errorText">{wizardErrors.description}</div>
                  ) : null}
                </div>

                {wizardType !== "project" ? (
                  <div className="field">
                    <label>Project *</label>
                    <select
                      value={projectId}
                      onChange={(e) => setProjectId(e.currentTarget.value)}
                    >
                      <option value="">Select project</option>
                      {projects.map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.title}
                        </option>
                      ))}
                    </select>
                    {wizardErrors.projectId ? (
                      <div className="errorText">{wizardErrors.projectId}</div>
                    ) : null}
                  </div>
                ) : null}

                {wizardType === "story" ? (
                  <div className="field">
                    <label>Epic</label>
                    <select
                      value={epicId}
                      onChange={(e) => setEpicId(e.currentTarget.value)}
                    >
                      <option value="">Select epic</option>
                      {epics
                        .filter((epic) =>
                          projectId ? epic.project_id === projectId : true
                        )
                        .map((epic) => (
                          <option key={epic.id} value={epic.id}>
                            {epic.title}
                          </option>
                        ))}
                    </select>
                  </div>
                ) : null}
              </div>
            ) : null}

            {wizardStep === 2 ? (
              <div className="wizardBody">
                {wizardType !== "story" ? (
                  <div className="hint">
                    Stories are the only items that appear on the board.
                  </div>
                ) : (
                  <>\n                    <div className="field inline">
                      <label>AI auto-fill</label>
                      <input
                        type="checkbox"
                        checked={useAiAutofill}
                        onChange={(e) => setUseAiAutofill(e.currentTarget.checked)}
                      />
                      <button
                        className="ghostButton"
                        onClick={handleAutoFill}
                        disabled={!useAiAutofill || isAutofilling}
                      >
                        {isAutofilling ? "Filling…" : "Auto-fill with AI"}
                      </button>
                    </div>

                    {aiError ? <div className="errorText">{aiError}</div> : null}

                    <div className="field">
                      <label>As a *</label>
                      <input
                        value={asA}
                        onChange={(e) => setAsA(e.currentTarget.value)}
                        placeholder="persona or user role"
                      />
                      {wizardErrors.asA ? (
                        <div className="errorText">{wizardErrors.asA}</div>
                      ) : null}
                    </div>

                    <div className="field">
                      <label>I want *</label>
                      <input
                        value={iWant}
                        onChange={(e) => setIWant(e.currentTarget.value)}
                        placeholder="the goal or capability"
                      />
                      {wizardErrors.iWant ? (
                        <div className="errorText">{wizardErrors.iWant}</div>
                      ) : null}
                    </div>

                    <div className="field">
                      <label>So that *</label>
                      <input
                        value={soThat}
                        onChange={(e) => setSoThat(e.currentTarget.value)}
                        placeholder="the outcome or benefit"
                      />
                      {wizardErrors.soThat ? (
                        <div className="errorText">{wizardErrors.soThat}</div>
                      ) : null}
                    </div>

                    <div className="field">
                      <label>Acceptance criteria</label>
                      {acceptanceCriteria.map((criterion, index) => (
                        <div key={index} className="criteriaRow">
                          <input
                            value={criterion}
                            onChange={(e) => {
                              const next = [...acceptanceCriteria];
                              next[index] = e.currentTarget.value;
                              setAcceptanceCriteria(next);
                            }}
                            placeholder={`Criterion ${index + 1}`}
                          />
                          <button
                            className="ghostButton"
                            onClick={() =>
                              setAcceptanceCriteria((prev) =>
                                prev.filter((_, i) => i !== index)
                              )
                            }
                          >
                            Remove
                          </button>
                        </div>
                      ))}
                      <button
                        className="ghostButton"
                        onClick={() =>
                          setAcceptanceCriteria((prev) => [...prev, ""])
                        }
                      >
                        Add criterion
                      </button>
                    </div>
                  </>
                )}
              </div>
            ) : null}

            <footer className="modalFooter">
              <button
                className="ghostButton"
                onClick={() => setWizardStep(Math.max(0, wizardStep - 1))}
              >
                Back
              </button>
              <div className="footerActions">
                <button className="ghostButton" onClick={closeWizard}>
                  Cancel
                </button>
                <button className="button" onClick={handleSubmitWizard}>
                  Create
                </button>
              </div>
            </footer>
          </div>
        </div>
      ) : null}
    </main>
  );
}
