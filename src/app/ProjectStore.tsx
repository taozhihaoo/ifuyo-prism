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
  getDiscovery,
  getInspectStatus,
  getScanStatus,
  openProject as invokeOpenProject,
  pickDirectory,
  startInspect as invokeStartInspect,
  startScan as invokeStartScan,
  toPrismError,
} from "../services/ipc";
import type {
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
  const [lastError, setLastError] = useState<string | null>(null);
  const pollingRef = useRef<number | null>(null);
  const inspectPollingRef = useRef<number | null>(null);

  const stopPolling = useCallback(() => {
    if (pollingRef.current !== null) {
      window.clearInterval(pollingRef.current);
      pollingRef.current = null;
    }
  }, []);

  const stopInspectPolling = useCallback(() => {
    if (inspectPollingRef.current !== null) {
      window.clearInterval(inspectPollingRef.current);
      inspectPollingRef.current = null;
    }
  }, []);

  /** 轮询直到扫描离开 running 态；结束时拉取 Discovery 并链式执行代码理解。 */
  const pollUntilDone = useCallback(async () => {
    stopPolling();
    const tick = async () => {
      try {
        const status = await getScanStatus();
        setScanStatus(status);
        if (status.status !== "running" && status.status !== "queued") {
          stopPolling();
          const result = await getDiscovery();
          setDiscovery(result);
          // 扫描完成（含 cancelled 保留部分结果）→ 自动链式执行 inspect。
          if (result) {
            try {
              await invokeStartInspect();
              void pollInspectUntilDone();
            } catch (chainErr) {
              setLastError(toPrismError(chainErr).message);
            }
          }
        }
      } catch (err) {
        stopPolling();
        setLastError(toPrismError(err).message);
      }
    };
    // 立即轮询一次，然后按间隔继续。
    void tick();
    pollingRef.current = window.setInterval(() => void tick(), POLL_INTERVAL_MS);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [stopPolling]);

  /** 轮询直到 inspect 离开 running 态；结束时拉取 Code Model。 */
  const pollInspectUntilDone = useCallback(async () => {
    stopInspectPolling();
    const tick = async () => {
      try {
        const status = await getInspectStatus();
        setInspectStatus(status);
        if (status.status !== "running") {
          stopInspectPolling();
          const model = await getCodeModel();
          setCodeModel(model);
        }
      } catch (err) {
        stopInspectPolling();
        setLastError(toPrismError(err).message);
      }
    };
    void tick();
    inspectPollingRef.current = window.setInterval(() => void tick(), POLL_INTERVAL_MS);
  }, [stopInspectPolling]);

  const openProject = useCallback(
    async (path: string) => {
      setLastError(null);
      const opened = await invokeOpenProject(path);
      setProject(opened);
      setDiscovery(null);
      setCodeModel(null);
      setInspectStatus(null);
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
      void pollUntilDone();
    },
    [pollUntilDone],
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
    await invokeStartScan();
    void pollUntilDone();
  }, [pollUntilDone]);

  const startInspect = useCallback(async () => {
    setLastError(null);
    await invokeStartInspect();
    void pollInspectUntilDone();
  }, [pollInspectUntilDone]);

  const cancelScan = useCallback(() => {
    void invokeCancelScan().catch((err) => setLastError(toPrismError(err).message));
  }, []);

  const closeProject = useCallback(() => {
    stopPolling();
    stopInspectPolling();
    setProject(null);
    setDiscovery(null);
    setScanStatus(null);
    setCodeModel(null);
    setInspectStatus(null);
    setLastError(null);
  }, [stopPolling, stopInspectPolling]);

  // 组件卸载时停止轮询。
  useEffect(
    () => () => {
      stopPolling();
      stopInspectPolling();
    },
    [stopPolling, stopInspectPolling],
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
