// 生成 Prism 品牌图标源图（1024x1024 PNG：深蓝底 + 青→紫渐变棱镜三角）。
// 仅构建期工具，使用 Node 内置 zlib 手写 PNG，不引入图像依赖。
// 运行：npm run icon（随后由 tauri icon 生成各平台尺寸）。
import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SIZE = 1024;
const OUT = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../assets/icon-source.png",
);

// ---------- PNG 编码 ----------
const CRC_TABLE = new Int32Array(256);
for (let n = 0; n < 256; n++) {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  CRC_TABLE[n] = c;
}
function crc32(buf) {
  let c = 0xffffffff;
  for (const b of buf) c = CRC_TABLE[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([len, typeBuf, data, crc]);
}

// ---------- 绘制 ----------
const clamp01 = (v) => Math.min(1, Math.max(0, v));
const lerp3 = (a, b, t) => [
  a[0] + (b[0] - a[0]) * t,
  a[1] + (b[1] - a[1]) * t,
  a[2] + (b[2] - a[2]) * t,
];

const BG_TOP = [15, 23, 42]; // #0f172a
const BG_BOTTOM = [30, 41, 59]; // #1e293b
const TRI_TOP = [34, 211, 238]; // #22d3ee
const TRI_BOTTOM = [139, 92, 246]; // #8b5cf6

const A = [512, 150];
const B = [140, 838];
const C = [884, 838];

function edgeSign(p, p1, p2) {
  return (p[0] - p2[0]) * (p1[1] - p2[1]) - (p1[0] - p2[0]) * (p[1] - p2[1]);
}
function inTriangle(p) {
  const d1 = edgeSign(p, A, B);
  const d2 = edgeSign(p, B, C);
  const d3 = edgeSign(p, C, A);
  const hasNeg = d1 < 0 || d2 < 0 || d3 < 0;
  const hasPos = d1 > 0 || d2 > 0 || d3 > 0;
  return !(hasNeg && hasPos);
}

const raw = Buffer.alloc(SIZE * (SIZE * 4 + 1));
let offset = 0;
for (let y = 0; y < SIZE; y++) {
  raw[offset++] = 0; // filter: none
  const bg = lerp3(BG_TOP, BG_BOTTOM, y / (SIZE - 1));
  for (let x = 0; x < SIZE; x++) {
    let color = bg;
    if (inTriangle([x, y])) {
      const k = clamp01((y - A[1]) / (B[1] - A[1]));
      color = lerp3(TRI_TOP, TRI_BOTTOM, k);
    }
    raw[offset++] = Math.round(color[0]);
    raw[offset++] = Math.round(color[1]);
    raw[offset++] = Math.round(color[2]);
    raw[offset++] = 255;
  }
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(SIZE, 0);
ihdr.writeUInt32BE(SIZE, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // color type: RGBA
ihdr[10] = 0; // compression
ihdr[11] = 0; // filter
ihdr[12] = 0; // interlace

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);

mkdirSync(dirname(OUT), { recursive: true });
writeFileSync(OUT, png);
console.log(`wrote ${OUT} (${png.length} bytes)`);
