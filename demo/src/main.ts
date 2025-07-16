import "./style.css";

let consumeKeyFlags: () => void;
let updateFn: ((x: number, y: number) => void) | null = null;
let startFn: (() => void) | null = null;
const canvas = document.getElementById("terminal-canvas") as HTMLCanvasElement;
const ctx = canvas.getContext("2d")!;
let mouseX = 0;
let mouseY = 0;
canvas.addEventListener("mousemove", (e) => {
  const rect = canvas.getBoundingClientRect();
  mouseX = Math.floor(e.clientX - rect.left);
  mouseY = Math.floor(e.clientY - rect.top);
});

const framebuffer = {
  fill_rectangle: (
    x: number,
    y: number,
    width: number,
    height: number,
    r: number,
    g: number,
    b: number,
    a: number,
  ) => {
    ctx.fillStyle = `rgba(${r},${g},${b},${a / 255})`;
    ctx.fillRect(x, y, width, height);
  },
  draw_line: (
    x0: number,
    y0: number,
    x1: number,
    y1: number,
    r: number,
    g: number,
    b: number,
    a: number,
  ) => {
    ctx.strokeStyle = `rgba(${r},${g},${b},${a / 255})`;
    ctx.beginPath();
    ctx.moveTo(x0, y0);
    ctx.lineTo(x1, y1);
    ctx.stroke();
  },
  draw_circle: (
    x: number,
    y: number,
    radius: number,
    r: number,
    g: number,
    b: number,
    a: number,
  ) => {
    ctx.strokeStyle = `rgba(${r},${g},${b},${a / 255})`;
    ctx.beginPath();
    ctx.arc(x, y, radius, 0, 2 * Math.PI);
    ctx.stroke();
  },
  fill_circle: (
    x: number,
    y: number,
    radius: number,
    r: number,
    g: number,
    b: number,
    a: number,
  ) => {
    ctx.fillStyle = `rgba(${r},${g},${b},${a / 255})`;
    ctx.beginPath();
    ctx.arc(x, y, radius, 0, 2 * Math.PI);
    ctx.fill();
  },
  fill_gradient: (
    x0: number,
    y0: number,
    x1: number,
    y1: number,
    r1: number,
    g1: number,
    b1: number,
    a1: number,
    r2: number,
    g2: number,
    b2: number,
    a2: number,
  ) => {
    const grad = ctx.createLinearGradient(x0, y0, x1, y1);
    grad.addColorStop(0, `rgba(${r1},${g1},${b1},${a1 / 255})`);
    grad.addColorStop(1, `rgba(${r2},${g2},${b2},${a2 / 255})`);
    ctx.fillStyle = grad;
    ctx.fillRect(x0, y0, x1 - x0, y1 - y0);
  },
  draw_triangle: (
    x1: number,
    y1: number,
    x2: number,
    y2: number,
    x3: number,
    y3: number,
    r: number,
    g: number,
    b: number,
    a: number,
  ) => {
    ctx.strokeStyle = `rgba(${r},${g},${b},${a / 255})`;
    ctx.beginPath();
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y2);
    ctx.lineTo(x3, y3);
    ctx.closePath();
    ctx.stroke();
  },
  draw_rectangle: (
    x: number,
    y: number,
    width: number,
    height: number,
    r: number,
    g: number,
    b: number,
    a: number,
  ) => {
    ctx.strokeStyle = `rgba(${r},${g},${b},${a / 255})`;
    ctx.strokeRect(x, y, width, height);
  },
  draw_rounded_rectangle: (
    x: number,
    y: number,
    width: number,
    height: number,
    radius: number,
    r: number,
    g: number,
    b: number,
    a: number,
  ) => {
    ctx.strokeStyle = `rgba(${r},${g},${b},${a / 255})`;
    ctx.beginPath();
    ctx.moveTo(x + radius, y);
    ctx.lineTo(x + width - radius, y);
    ctx.quadraticCurveTo(x + width, y, x + width, y + radius);
    ctx.lineTo(x + width, y + height - radius);
    ctx.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
    ctx.lineTo(x + radius, y + height);
    ctx.quadraticCurveTo(x, y + height, x, y + height - radius);
    ctx.lineTo(x, y + radius);
    ctx.quadraticCurveTo(x, y, x + radius, y);
    ctx.closePath();
    ctx.stroke();
  },
  set_pixel: (
    x: number,
    y: number,
    r: number,
    g: number,
    b: number,
    a: number,
  ) => {
    ctx.fillStyle = `rgba(${r},${g},${b},${a / 255})`;
    ctx.fillRect(x, y, 1, 1);
  },
  set_pixels_from_to: (
    x0: number,
    y0: number,
    x1: number,
    y1: number,
    r: number,
    g: number,
    b: number,
    a: number,
  ) => {
    ctx.fillStyle = `rgba(${r},${g},${b},${a / 255})`;
    for (let y = y0; y < y1; y++) {
      for (let x = x0; x < x1; x++) {
        ctx.fillRect(x, y, 1, 1);
      }
    }
  },
};

async function initWasm() {
  const response = await fetch("/terminal_app.wasm");
  const bytes = await response.arrayBuffer();
  // Keyboard state tracking
  const keyState: { [key: number]: boolean } = {};
  const keyPressed: { [key: number]: boolean } = {};
  const keyReleased: { [key: number]: boolean } = {};
  const keyHistory: Array<{ key: number; pressed: boolean }> = [];
  const MAX_HISTORY = 64;

  globalThis.addEventListener("keydown", (e) => {
    const code = e.keyCode;
    if (!keyState[code]) {
      keyPressed[code] = true;
      keyHistory.push({ key: code, pressed: true });
      if (keyHistory.length > MAX_HISTORY) keyHistory.shift();
    }
    keyState[code] = true;
  });
  globalThis.addEventListener("keyup", (e) => {
    const code = e.keyCode;
    keyState[code] = false;
    keyReleased[code] = true;
    keyHistory.push({ key: code, pressed: false });
    if (keyHistory.length > MAX_HISTORY) keyHistory.shift();
  });

  consumeKeyFlags = () => {
    for (const code in keyPressed) keyPressed[code] = false;
    for (const code in keyReleased) keyReleased[code] = false;
  };

  const imports = {
    agave: {
      fill_rectangle: framebuffer.fill_rectangle,
      draw_line: framebuffer.draw_line,
      draw_circle: framebuffer.draw_circle,
      fill_circle: framebuffer.fill_circle,
      fill_gradient: framebuffer.fill_gradient,
      draw_triangle: framebuffer.draw_triangle,
      draw_rectangle: framebuffer.draw_rectangle,
      draw_rounded_rectangle: framebuffer.draw_rounded_rectangle,
      set_pixel: framebuffer.set_pixel,
      set_pixels_from_to: framebuffer.set_pixels_from_to,
      get_width: () => 1491,
      get_height: () => 900,
      get_time_ms: () => BigInt(Date.now()),
      grow_memory: (_pages: number) => 1,
      is_key_down: (keyCode: number) => keyState[keyCode] ? 1 : 0,
      is_key_pressed: (keyCode: number) => keyPressed[keyCode] ? 1 : 0,
      is_key_released: (keyCode: number) => keyReleased[keyCode] ? 1 : 0,
      get_key_history_count: () => keyHistory.length,
      get_key_history_event: (index: number) => {
        if (index >= 0 && index < keyHistory.length) {
          const event = keyHistory[index];
          // Pack key code in low 32 bits, pressed state in high 32 bits
          return BigInt((event.pressed ? (1 << 32) : 0) | event.key);
        }
        return BigInt(0);
      },
    },
    wasi_snapshot_preview1: {
      fd_write: () => 0,
      environ_get: () => 0,
      environ_sizes_get: () => 0,
      proc_exit: () => {
        throw new Error("WASM exit");
      },
    },
  };
  const { instance } = await WebAssembly.instantiate(bytes, imports);
  const startExport = instance.exports.start;
  const updateExport = instance.exports.update;
  if (typeof startExport === "function") {
    startFn = startExport as () => void;
  }
  if (typeof updateExport === "function") {
    updateFn = updateExport as (x: number, y: number) => void;
  }
  requestAnimationFrame(frame);
}

function frame() {
  if (updateFn) {
    updateFn(mouseX, mouseY);
    consumeKeyFlags();
  }
  requestAnimationFrame(frame);
}


initWasm();
if (startFn !== null) {
  startFn();
}
