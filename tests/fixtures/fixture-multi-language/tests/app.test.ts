import { helper } from "../src/utils";

// 占位测试：供 M8 verify 测试发现使用，不在前端/CI 执行。
describe("fixture-multi-language", () => {
  it("calls python-shaped helper", () => {
    if (typeof helper("x") !== "string") {
      throw new Error("fixture assertion failed");
    }
  });
});
