import { useProjectStore } from "../app/ProjectStore";
import DirectoryTree from "../components/DirectoryTree";
import FileList from "../components/FileList";
import PlaceholderPage from "../components/PlaceholderPage";

/**
 * Understand：项目结构（树 + 文件列表）。
 * 需要先在 Overview 打开并扫描项目；未扫描时如实说明（纲领 §75）。
 */
export default function UnderstandPage() {
  const { project, discovery } = useProjectStore();

  if (!project || !discovery) {
    return (
      <PlaceholderPage
        milestone="M1"
        description="Open and scan a project on the Overview page to see its directory tree and file list here."
      />
    );
  }

  return (
    <section className="stack">
      <div className="card">
        <h2 className="card-title">Project tree</h2>
        <p className="muted small">
          {discovery.project.name} — {discovery.statistics.total_directories} directories,{" "}
          {discovery.statistics.total_files} files (ignored entries are not listed).
        </p>
        <DirectoryTree directories={discovery.directories} files={discovery.files} />
      </div>
      <FileList files={discovery.files} />
    </section>
  );
}
