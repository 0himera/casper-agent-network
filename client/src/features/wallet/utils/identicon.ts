function hashFromHex(hex: string): number[] {
  const h: number[] = [];
  for (let i = 0; i < hex.length && h.length < 64; i += 2) {
    h.push(parseInt(hex.substring(i, i + 2), 16));
  }
  while (h.length < 64) h.push(0);
  return h;
}

const COLORS = [
  "#6366f1", "#8b5cf6", "#a855f7", "#d946ef",
  "#ec4899", "#f43f5e", "#f97316", "#eab308",
];

export function generateIdenticonSvg(hex: string, size: number): string {
  const hash = hashFromHex(hex);
  const cell = size / 8;
  const color = COLORS[hash[0] % COLORS.length];
  const bg = hash[1] % 2 === 0 ? "#1e1e24" : "#2a2a32";

  type Rect = [number, number];
  const rects: Rect[] = [];
  for (let row = 0; row < 8; row++) {
    const mirror = row < 4 ? row : 7 - row;
    for (let col = 0; col <= mirror; col++) {
      const idx = row * 8 + col;
      if (hash[idx % hash.length] % 2 === 0) {
        rects.push([col * cell, row * cell]);
        if (col !== mirror) rects.push([(7 - col) * cell, row * cell]);
      }
    }
  }

  const svgRects = rects
    .map(([x, y]) => `<rect x="${x}" y="${y}" width="${cell}" height="${cell}" rx="1"/>`)
    .join("");
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}">
    <rect width="${size}" height="${size}" rx="${size * 0.15}" fill="${bg}"/>
    <g fill="${color}" opacity="0.85">${svgRects}</g>
  </svg>`;
}
