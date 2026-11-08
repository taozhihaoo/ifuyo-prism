/**
 * ProjectStore：M1 的集中式项目/扫描状态（纲领 §67）。
 *
 * 状态机：NoProject → Selecting → Scanning → Completed/CompletedWithWarnings
 * / Failed / Cancelled。扫描进度经 IPC 轮询（400ms），数据全部来自真实
 * scanner，禁止伪造进度。
 */
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  cancelScan as invokeCancelScan,
  getCodeModel,
  getDependencyGraph,
  getDiscovery,
  getGraphStatus,
  getInspectStatus,
  getScanStatus,
  openProject as invokeOpenProject,
  pickDirectory,
  startGraph as invokeStartGraph,
  startInspect as invokeStartInspect,
  startScan as invokeStartScan,
  toPrismError,
} from "../services/ipc";
import type {
  DependencyGraphResultDto,
  GraphStatus,
  InspectStatus,
  ProjectCodeModelDto,
  ProjectDiscovery,
  ProjectInfo,
  ScanStatus,
} from "../types/prism";

export type ScanPhase =
  | "noProject"
  | "idle"
  | "scanning"
  | "analyzing"
  | "completed"
  | "completedWithWarnings"
  | "failed"
  | "cancelled";

interface ProjectStoreValue {
  project: ProjectInfo | null;
  phase: ScanPhase;
  scanStatus: ScanStatus | null;
  discovery: ProjectDiscovery | null;
  inspectStatus: InspectStatus | null;
  codeModel: ProjectCodeModelDto | null;
  graphStatus: GraphStatus | null;
  dependencyGraph: DependencyGraphResultDto | null;
  lastError: string | null;
  /** 打开系统目录选择对话框并打开所选项目。 */
  pickAndOpenProject: () => Promise<void>;
  /** 直接按路径打开项目（测试/CLI 同源入口）。 */
  openProject: (path: string) => Promise<void>;
  startScan: () => Promise<void>;
  /** 扫描完成后手动（重）触发代码理解。 */
  startInspect: () => Promise<void>;
  cancelScan: () => void;
  /** 关闭当前项目回到空状态。 */
  closeProject: () => void;
}

const ProjectStoreContext = createContext<ProjectStoreValue | null>(null);

const POLL_INTERVAL_MS = 400;

export function ProjectProvider({ children }: { children: ReactNode }) {
  const [project, setProject] = useState<ProjectInfo | null>(null);
  const [scanStatus, setScanStatus] = useState<ScanStatus | null>(null);
  const [discovery, setDiscovery] = useState<ProjectDiscovery | null>(null);
  const [inspectStatus, setInspectStatus] = useState<InspectStatus | null>(null);
  const [codeModel, setCodeModel] = useState<ProjectCodeModelDto | null>(null);
  const [graphStatus, setGraphStatus] = useState<GraphStatus | null>(null);
  const [dependencyGraph, setDependencyGraph] = useState<DependencyGraphResultDto | null>(null);
  const [lastError, setLastError] = useState<string | null>(null);
  const pollingRef = useRef<number | null>(null);
  const inspectPollingRef = useRef<number | null>(null);
  const graphPollingRef = useRef<number | null>(null);
  /** 每次打开/关闭项目递增；旧轮询发现自己过期后停止并丢弃结果。 */
  const generationRef = useRef(0);

  const stopPolling = useCallback(() => {
    if (pollingRef.current !== null) {
      window.clearTimeout(pollingRef.current);
      pollingRef.current = null;
    }
  }, []);

  const stopInspectPolling = useCallback(() => {
    if (inspectPollingRef.current !== null) {
      window.clearTimeout(inspectPollingRef.current);
      inspectPollingRef.current = null;
    }
  }, []);

  const stopGraphPolling = useCallback(() => {
    if (graphPollingRef.current !== null) {
      window.clearTimeout(graphPollingRef.current);
      graphPollingRef.current = null;
    }
  }, []);

  /**
   * 轮询改为「链式 setTimeout + 单次在飞」：上一轮未返回前不会安排下一轮，
   * 避免 400ms interval 叠加导致重复 start_inspect / start_graph。
   * 所有回调都校验 generation，切换项目后旧会话结果不会写回新项目。
   */
  const pollGraphUntilDone = useCallback(() => {
    stopGraphPolling();
    const generation = generationRef.current;

    function schedule() {
      graphPollingRef.current = window.setTimeout(() => void tick(), POLL_INTERVAL_MS);
    }

    async function tick() {
      if (generation !== generationRef.current) {
        stopGraphPolling();
        return;
      }
      try {
        const status = await getGraphStatus();
        if (generation !== generationRef.current) return;
        setGraphStatus(status);
        if (status.status !== "running") {
          stopGraphPolling();
          const graph = await getDependencyGraph();
          if (generation !== generationRef.current) return;
          setDependencyGraph(graph);
          return;
        }
        schedule();
      } catch (err) {
        stopGraphPolling();
        if (generation === generationRef.current) setLastError(toPrismError(err).message);
      }
    }

    void tick();
  }, [stopGraphPolling]);

  const pollInspectUntilDone = useCallback(() => {
    stopInspectPolling();
    const generation = generationRef.current;

    function schedule() {
      inspectPollingRef.current = window.setTimeout(() => void tick(), POLL_INTERVAL_MS);
    }

    async function tick() {
      if (generation !== generationRef.current) {
        stopInspectPolling();
        return;
      }
      try {
        const status = await getInspectStatus();
        if (generation !== generationRef.current) return;
        setInspectStatus(status);
        if (status.status !== "running") {
          stopInspectPolling();
          const model = await getCodeModel();
          if (generation !== generationRef.current) return;
          setCodeModel(model);
          // inspect 完成 → 链式执行依赖图分析（M3）。
          try {
            await invokeStartGraph();
            if (generation === generationRef.current) pollGraphUntilDone();
          } catch (chainErr) {
            if (generation === generationRef.current) {
              setLastError(toPrismError(chainErr).message);
            }
          }
          return;
        }
        schedule();
      } catch (err) {
        stopInspectPolling();
        if (generation === generationRef.current) setLastError(toPrismError(err).message);
      }
    }

    void tick();
  }, [stopInspectPolling, pollGraphUntilDone]);

  const pollUntilDone = useCallback(() => {
    stopPolling();
    const generation = generationRef.current;

    function schedule() {
      pollingRef.current = window.setTimeout(() => void tick(), POLL_INTERVAL_MS);
    }

    async function tick() {
      if (generation !== generationRef.current) {
        stopPolling();
        return;
      }
      try {
        const status = await getScanStatus();
        if (generation !== generationRef.current) return;
        setScanStatus(status);
        if (status.status !== "running" && status.status !== "queued") {
          stopPolling();
          const result = await getDiscovery();
          if (generation !== generationRef.current) return;
          setDiscovery(result);
          // 扫描完成（含 cancelled 保留部分结果）→ 自动链式执行 inspect。
          if (result) {
            try {
              await invokeStartInspect();
              if (generation === generationRef.current) pollInspectUntilDone();
            } catch (chainErr) {
              if (generation === generationRef.current) {
                setLastError(toPrismError(chainErr).message);
              }
            }
          }
          return;
        }
        schedule();
      } catch (err) {
        stopPolling();
        if (generation === generationRef.current) setLastError(toPrismError(err).message);
      }
    }

    void tick();
  }, [stopPolling, pollInspectUntilDone]);

  const openProject = useCallback(
    async (path: string) => {
      // 切换项目：先停掉旧项目的三条轮询链，再作废旧会话的所有在飞结果。
      stopPolling();
      stopInspectPolling();
      stopGraphPolling();
      generationRef.current += 1;
      const generation = generationRef.current;
      setLastError(null);
      try {
        const opened = await invokeOpenProject(path);
        if (generation !== generationRef.current) return;
        setProject(opened);
        setDiscovery(null);
        setCodeModel(null);
        setInspectStatus(null);
        setGraphStatus(null);
        setDependencyGraph(null);
        setScanStatus({
          project: opened,
          status: "idle",
          directories: 0,
          files: 0,
          ignored: 0,
          errors: 0,
          has_warnings: false,
          error: null,
        });
        await invokeStartScan();
        if (generation === generationRef.current) pollUntilDone();
      } catch (err) {
        if (generation !== generationRef.current) return;
        const message = toPrismError(err).message;
        setLastError(message);
        // 启动失败时不留下「看起来在扫描」的假状态（§46）。
        setScanStatus((previous) => ({
          project: previous?.project ?? null,
          status: "failed",
          directories: previous?.directories ?? 0,
          files: previous?.files ?? 0,
          ignored: previous?.ignored ?? 0,
          errors: previous?.errors ?? 0,
          has_warnings: previous?.has_warnings ?? false,
          error: message,
        }));
      }
    },
    [stopPolling, stopInspectPolling, stopGraphPolling, pollUntilDone],
  );

  const pickAndOpenProject = useCallback(async () => {
    setLastError(null);
    const path = await pickDirectory();
    if (!path) {
      return; // 用户取消选择：留在原状态
    }
    await openProject(path);
  }, [openProject]);

  const startScan = useCallback(async () => {
    try {
      await invokeStartScan();
      pollUntilDone();
    } catch (err) {
      setLastError(toPrismError(err).message);
    }
  }, [pollUntilDone]);

  const startInspect = useCallback(async () => {
    setLastError(null);
    try {
      await invokeStartInspect();
      pollInspectUntilDone();
    } catch (err) {
      setLastError(toPrismError(err).message);
    }
  }, [pollInspectUntilDone]);

  const cancelScan = useCallback(() => {
    void invokeCancelScan().catch((err) => setLastError(toPrismError(err).message));
  }, []);

  const closeProject = useCallback(() => {
    stopPolling();
    stopInspectPolling();
    stopGraphPolling();
    generationRef.current += 1;
    setProject(null);
    setDiscovery(null);
    setScanStatus(null);
    setCodeModel(null);
    setInspectStatus(null);
    setGraphStatus(null);
    setDependencyGraph(null);
    setLastError(null);
  }, [stopPolling, stopInspectPolling, stopGraphPolling]);

  // 组件卸载时停止轮询。
  useEffect(
    () => () => {
      generationRef.current += 1;
      stopPolling();
      stopInspectPolling();
      stopGraphPolling();
    },
    [stopPolling, stopInspectPolling, stopGraphPolling],
  );

  const phase: ScanPhase = useMemo(() => {
    if (!project) return "noProject";
    if (scanStatus?.status === "running" || scanStatus?.status === "queued") return "scanning";
    if (inspectStatus?.status === "running") return "analyzing";
    if (discovery) {
      if (discovery.scan.status === "cancelled") return "cancelled";
      if (discovery.scan.errors_count > 0) return "completedWithWarnings";
      return "completed";
    }
    if (scanStatus?.status === "failed") return "failed";
    return "idle";
  }, [project, scanStatus, inspectStatus, discovery]);

  const value = useMemo<ProjectStoreValue>(
    () => ({
      project,
      phase,
      scanStatus,
      discovery,
      inspectStatus,
      codeModel,
      graphStatus,
      dependencyGraph,
      lastError,
      pickAndOpenProject,
      openProject,
      startScan,
      startInspect,
      cancelScan,
      closeProject,
    }),
    [
      project,
      phase,
      scanStatus,
      discovery,
      inspectStatus,
      codeModel,
      graphStatus,
      dependencyGraph,
      lastError,
      pickAndOpenProject,
      openProject,
      startScan,
      startInspect,
      cancelScan,
      closeProject,
    ],
  );

  return <ProjectStoreContext.Provider value={value}>{children}</ProjectStoreContext.Provider>;
}

export function useProjectStore(): ProjectStoreValue {
  const value = useContext(ProjectStoreContext);
  if (!value) {
    throw new Error("useProjectStore must be used within ProjectProvider");
  }
  return value;
}
