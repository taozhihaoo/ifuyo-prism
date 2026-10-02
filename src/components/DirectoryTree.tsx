import { useMemo } from "react";
import type { DirectoryInfoDto, FileInfoDto } from "../types/prism";

interface TreeNode {
  name: string;
  children: Map<string, TreeNode>;
  files: FileInfoDto[];
}

function buildTree(directories: DirectoryInfoDto[], files: FileInfoDto[]): TreeNode {
  const root: TreeNode = { name: "", children: new Map(), files: [] };
  for (const directory of directories) {
    let node = root;
    for (const segment of directory.relative_path.split("/")) {
      let child = node.children.get(segment);
      if (!child) {
        child = { name: segment, children: new Map(), files: [] };
        node.children.set(segment, child);
      }
      node = child;
    }
  }
  for (const file of files) {
    let node = root;
    const segments = file.relative_path.split("/");
    let ok = true;
    for (const segment of segments.slice(0, -1)) {
      const child = node.children.get(segment);
      if (!child) {
        ok = false; // 防御：目录缺失时不虚构
        break;
      }
      node = child;
    }
    if (ok) node.files.push(file);
  }
  return root;
}

function sortNodes(nodes: TreeNode[]): TreeNode[] {
  return [...nodes].sort((a, b) => a.name.localeCompare(b.name));
}

/** 基础项目树：只展示真实发现结果，不做内容预览（纲领 §43）。 */
export default function DirectoryTree({
  directories,
  files,
}: {
  directories: DirectoryInfoDto[];
  files: FileInfoDto[];
}) {
  const root = useMemo(() => buildTree(directories, files), [directories, files]);

  const renderNode = (node: TreeNode): JSX.Element => {
    const children = sortNodes([...node.children.values()]);
    const fileNames = node.files.map((file) => file.file_name).sort();
    return (
      <ul className="tree-branch" role="group">
        {children.map((child) => (
          <li key={child.name} role="treeitem">
            <span className="tree-dir">{child.name}/</span>
            {renderNode(child)}
          </li>
        ))}
        {fileNames.map((name) => (
          <li key={name} role="treeitem" className="tree-file">
            {name}
          </li>
        ))}
      </ul>
    );
  };

  return (
    <div data-testid="directory-tree" className="tree">
      {renderNode(root)}
    </div>
  );
}
