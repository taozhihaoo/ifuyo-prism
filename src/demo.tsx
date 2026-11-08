/**
 * 浏览器内 UI 预览入口（`demo.html`）。仅用于本地查看/截图：安装演示数据
 * 的 IPC shim，渲染与桌面端完全相同的 React 应用。
 *
 * 用法：`npm run dev` 后打开
 *   http://127.0.0.1:5173/demo.html?lang=en
 *   http://127.0.0.1:5173/demo.html?lang=zh
 */
import ReactDOM from "react-dom/client";
import App from "./app/App";
import "./app/styles.css";
import { applyDemoLanguage, installDemoIpc } from "./app/demoIpc";

applyDemoLanguage();
installDemoIpc();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(<App />);

/** 等空状态渲染出来后自动“点开”演示项目，让面板有真实内容可看。 */
function autoOpenDemoProject(attempt = 0): void {
  const button = document.querySelector<HTMLButtonElement>(
    '[data-testid="empty-state"] button',
  );
  if (button) {
    button.click();
    return;
  }
  if (attempt < 40) {
    window.setTimeout(() => autoOpenDemoProject(attempt + 1), 50);
  }
}

window.setTimeout(() => autoOpenDemoProject(), 60);
