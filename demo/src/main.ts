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

  const ENVIRON = ["PATH=/usr/bin", "HOME=/root", "USER=agave"];
  const ENVIRON_BUF = ENVIRON.map((e) => e + "\0").join("");
  const encoder = new TextEncoder();

  // deno-lint-ignore prefer-const
  let instance: WebAssembly.Instance;
  const wasi_snapshot_preview1 = {
    fd_readdir: (
      _fd: number,
      _buf: number,
      _buf_len: number,
      _cookie: bigint,
      bufused_ptr: number
    ) => {
      // Always return 0 bytes used, success
      const memory = instance.exports.memory as WebAssembly.Memory;
      new DataView(memory.buffer).setUint32(bufused_ptr, 0, true);
      return 0;
    },
    fd_read: (
      _fd: number,
      _iovs: number,
      _iovs_len: number,
      nread_ptr: number
    ) => {
      // Always return 0 bytes read, success
      const memory = instance.exports.memory as WebAssembly.Memory;
      new DataView(memory.buffer).setUint32(nread_ptr, 0, true);
      return 0;
    },
    fd_write: (
      fd: number,
      iovs: number,
      iovs_len: number,
      nwritten_ptr: number,
    ) => {
      const memory = instance.exports.memory as WebAssembly.Memory;
      const view = new DataView(memory.buffer);
      let written = 0;
      for (let i = 0; i < iovs_len; i++) {
        const ptr = view.getUint32(iovs + i * 8, true);
        const len = view.getUint32(iovs + i * 8 + 4, true);
        const bytes = new Uint8Array(memory.buffer, ptr, len);
        const text = new TextDecoder().decode(bytes);
        if (fd === 1 || fd === 2) console.log(text);
        written += len;
      }
      view.setUint32(nwritten_ptr, written, true);
      return 0;
    },
    environ_get: (environ: number, environ_buf: number) => {
      const memory = instance.exports.memory as WebAssembly.Memory;
      const view = new DataView(memory.buffer);
      let buf_offset = 0;
      for (let i = 0; i < ENVIRON.length; i++) {
        view.setUint32(environ + i * 4, environ_buf + buf_offset, true);
        const bytes = encoder.encode(ENVIRON[i] + "\0");
        new Uint8Array(memory.buffer, environ_buf + buf_offset, bytes.length)
          .set(bytes);
        buf_offset += bytes.length;
      }
      return 0;
    },
    environ_sizes_get: (count_ptr: number, buf_size_ptr: number) => {
      const memory = instance.exports.memory as WebAssembly.Memory;
      const view = new DataView(memory.buffer);
      view.setUint32(count_ptr, ENVIRON.length, true);
      view.setUint32(buf_size_ptr, ENVIRON_BUF.length, true);
      return 0;
    },
    proc_exit: (code: number) => {
      throw new Error("WASM exit: " + code);
    },
    fd_close: (_fd: number) => 0,
    fd_fdstat_get: (_fd: number, stat_ptr: number) => {
      const memory = instance.exports.memory as WebAssembly.Memory;
      new Uint8Array(memory.buffer, stat_ptr, 24).fill(0);
      return 0;
    },
    fd_prestat_get: (_fd: number, prestat_ptr: number) => {
      const memory = instance.exports.memory as WebAssembly.Memory;
      new Uint8Array(memory.buffer, prestat_ptr, 8).fill(0);
      return 0;
    },
    fd_prestat_dir_name: (_fd: number, path: number, path_len: number) => {
      const memory = instance.exports.memory as WebAssembly.Memory;
      new Uint8Array(memory.buffer, path, path_len).fill(0);
      return 0;
    },
    sched_yield: () => 0,
    fd_filestat_get: (_fd: number, filestat_ptr: number) => {
      const memory = instance.exports.memory as WebAssembly.Memory;
      new Uint8Array(memory.buffer, filestat_ptr, 56).fill(0);
      return 0;
    },
    fd_filestat_set_size: (_fd: number, _size: bigint) => 0,
    fd_fdstat_set_flags: (_fd: number, _flags: number) => 0,
    fd_sync: (_fd: number) => 0,
    fd_datasync: (_fd: number) => 0,
    fd_allocate: (_fd: number, _offset: bigint, _len: bigint) => 0,
    fd_advise: (_fd: number, _offset: bigint, _len: bigint, _advice: number) =>
      0,
    fd_tell: (_fd: number, offset_ptr: number) => {
      const memory = instance.exports.memory as WebAssembly.Memory;
      new DataView(memory.buffer).setBigUint64(offset_ptr, BigInt(0), true);
      return 0;
    },
    fd_seek: (
      _fd: number,
      _offset: bigint,
      _whence: number,
      new_offset_ptr: number,
    ) => {
      const memory = instance.exports.memory as WebAssembly.Memory;
      new DataView(memory.buffer).setBigUint64(new_offset_ptr, BigInt(0), true);
      return 0;
    },
    path_create_directory: (
      _fd: number,
      _path_ptr: number,
      _path_len: number,
    ) => 0,
    path_filestat_get: (
      _fd: number,
      _flags: number,
      _path_ptr: number,
      _path_len: number,
      filestat_ptr: number,
    ) => {
      const memory = instance.exports.memory as WebAssembly.Memory;
      new Uint8Array(memory.buffer, filestat_ptr, 56).fill(0);
      return 0;
    },
    path_filestat_set_times: (
      _fd: number,
      _flags: number,
      _path_ptr: number,
      _path_len: number,
      _atim: bigint,
      _mtim: bigint,
      _fst_flags: number,
    ) => 0,
    path_link: (
      _old_fd: number,
      _old_flags: number,
      _old_path_ptr: number,
      _old_path_len: number,
      _new_fd: number,
      _new_path_ptr: number,
      _new_path_len: number,
    ) => 0,
    path_open: (
      _fd: number,
      _dirflags: number,
      _path_ptr: number,
      _path_len: number,
      _oflags: number,
      _fs_rights_base: bigint,
      _fs_rights_inheriting: bigint,
      _fdflags: number,
      opened_fd_ptr: number,
    ) => {
      const memory = instance.exports.memory as WebAssembly.Memory;
      new DataView(memory.buffer).setUint32(opened_fd_ptr, 3, true);
      return 0;
    },
    path_readlink: (
      _fd: number,
      _path_ptr: number,
      _path_len: number,
      _buf_ptr: number,
      _buf_len: number,
      nread_ptr: number,
    ) => {
      const memory = instance.exports.memory as WebAssembly.Memory;
      new DataView(memory.buffer).setUint32(nread_ptr, 0, true);
      return 0;
    },
    path_remove_directory: (
      _fd: number,
      _path_ptr: number,
      _path_len: number,
    ) => 0,
    path_rename: (
      _fd: number,
      _old_path_ptr: number,
      _old_path_len: number,
      _new_fd: number,
      _new_path_ptr: number,
      _new_path_len: number,
    ) => 0,
    path_symlink: (
      _old_path_ptr: number,
      _old_path_len: number,
      _fd: number,
      _new_path_ptr: number,
      _new_path_len: number,
    ) => 0,
    path_unlink_file: (_fd: number, _path_ptr: number, _path_len: number) => 0,
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
          return BigInt((event.pressed ? (1 << 32) : 0) | event.key);
        }
        return BigInt(0);
      },
    },
    wasi_snapshot_preview1,
  };
  const wasmResult = await WebAssembly.instantiate(bytes, imports);
  instance = wasmResult.instance;
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
  (startFn as () => void)();
}
