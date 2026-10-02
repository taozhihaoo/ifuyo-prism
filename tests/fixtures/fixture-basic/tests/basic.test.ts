import { add } from "../src/utils";

// 占位测试：fixture 只需要稳定的测试文件结构（M8 的 verify 测试发现会用到）。
describe("fixture-basic", () => {
  it("adds numbers", () => {
    if (add(1, 2) !== 3) {
      throw new Error("fixture assertion failed");
    }
  });
});
