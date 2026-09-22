// Runs inside the core's dedicated Worker. SAH-pool files are private to this
// origin and one active Worker; a second owner fails to acquire the OPFS locks.
let sqlitePromise;
const databases = new Map();
let nextDatabase = 1;
const pendingDatabases = new Set();
export function databaseReserve() {
  const id = nextDatabase++;
  pendingDatabases.add(id);
  return id;
}
async function sqlite() {
  if (!sqlitePromise) {
    sqlitePromise = (async () => {
      const { default: initialize } = await import("@sqlite.org/sqlite-wasm");
      const sqlite3 = await initialize();
      const pool = await sqlite3.installOpfsSAHPoolVfs({
        name: "mixin",
        directory: ".mixin-sqlite",
        initialCapacity: 32,
      });
      return { sqlite3, pool };
    })().catch((error) => {
      sqlitePromise = undefined;
      throw error;
    });
  }
  return sqlitePromise;
}
export async function databaseOpen(id, path) {
  const { pool } = await sqlite();
  if (!pendingDatabases.delete(id)) throw new Error("Database open cancelled");
  const db = new pool.OpfsSAHPoolDb(path);
  databases.set(id, db);
  return id;
}
function decode(value) {
  switch (value.type) {
    case "Null":
      return null;
    case "Integer":
      return BigInt(value.value);
    case "Real":
    case "Text":
      return value.value;
    case "Blob":
      return new Uint8Array(value.value);
    default:
      throw new Error("Unknown database value");
  }
}
function encode(value) {
  if (value === null) return { type: "Null" };
  if (typeof value === "bigint")
    return { type: "Integer", value: value.toString() };
  if (typeof value === "number")
    return Number.isInteger(value)
      ? { type: "Integer", value: value.toString() }
      : { type: "Real", value };
  if (typeof value === "string") return { type: "Text", value };
  if (value instanceof Uint8Array)
    return { type: "Blob", value: Array.from(value) };
  throw new Error("Unknown SQLite value");
}
export function databaseExecute(handle, sql, parameters) {
  const db = databases.get(handle);
  if (!db) throw new Error("Database closed");
  const columns = [];
  const rows = [];
  db.exec({
    sql,
    bind: JSON.parse(parameters).map(decode),
    columnNames: columns,
    rowMode: "array",
    callback: (row) => {
      rows.push(row.map(encode));
    },
  });
  return JSON.stringify({
    columns,
    rows,
    changes: db.changes(),
    rowid: String(db.selectValue("SELECT last_insert_rowid()")),
  });
}
export function databaseClose(handle) {
  pendingDatabases.delete(handle);
  const db = databases.get(handle);
  databases.delete(handle);
  db?.close();
}

// Files are stored separately from SQLite. Handle acquisition and directory
// mutations are asynchronous; only reads/writes on an acquired handle are sync.
function components(path) {
  const parts = [];
  for (const part of path.split("/")) {
    if (!part || part === ".") continue;
    if (part === "..") {
      if (!parts.length) throw new Error("Path escapes storage");
      parts.pop();
    } else parts.push(part);
  }
  return parts;
}
async function directory(path, create = false) {
  let dir = await navigator.storage.getDirectory();
  dir = await dir.getDirectoryHandle("mixin-files", { create: true });
  for (const part of components(path))
    dir = await dir.getDirectoryHandle(part, { create });
  return dir;
}
async function parent(path, create = false) {
  const parts = components(path);
  const name = parts.pop();
  if (!name) throw new Error("Expected file name");
  return [await directory(parts.join("/"), create), name];
}
const files = new Map();
let nextFile = 1;
const pendingFiles = new Set();
export function fileReserve() {
  const id = nextFile++;
  pendingFiles.add(id);
  return id;
}
export async function fileOpen(id, path, mode) {
  const [dir, name] = await parent(path, mode !== "read");
  if (mode === "write-new") {
    try {
      await dir.getFileHandle(name);
      throw new Error("File already exists");
    } catch (error) {
      if (error.name !== "NotFoundError") throw error;
    }
  }
  const file = await dir.getFileHandle(name, { create: mode !== "read" });
  if (!pendingFiles.has(id)) throw new Error("File open cancelled");
  const access = await file.createSyncAccessHandle();
  if (!pendingFiles.delete(id)) {
    access.close();
    throw new Error("File open cancelled");
  }
  try {
    if (mode !== "read") access.truncate(0);
    files.set(id, access);
  } catch (error) {
    access.close();
    throw error;
  }
  return id;
}
export function fileRead(handle, offset, length) {
  const data = new Uint8Array(length);
  const count = files.get(handle).read(data, { at: Number(offset) });
  return data.subarray(0, count);
}
export function fileWrite(handle, offset, data) {
  return files.get(handle).write(data, { at: Number(offset) });
}
export function fileFlush(handle) {
  files.get(handle).flush();
}
export function fileClose(handle) {
  pendingFiles.delete(handle);
  const access = files.get(handle);
  files.delete(handle);
  access?.close();
}
export function fileSize(handle) {
  return files.get(handle).getSize();
}
export async function fileMetadata(path) {
  const [dir, name] = await parent(path);
  try {
    const file = await (await dir.getFileHandle(name)).getFile();
    return JSON.stringify({ len: file.size, is_file: true });
  } catch (error) {
    if (error.name !== "TypeMismatchError") throw error;
    await dir.getDirectoryHandle(name);
    return JSON.stringify({ len: 0, is_file: false });
  }
}
export async function createDirectory(path) {
  await directory(path, true);
}
export async function removePath(path, recursive) {
  const [dir, name] = await parent(path);
  await dir.removeEntry(name, { recursive });
}
export async function moveFile(from, to) {
  const [source, name] = await parent(from);
  const file = await source.getFileHandle(name);
  const [target, targetName] = await parent(to, true);
  await file.move(target, targetName);
}
export async function listDirectory(path) {
  const result = [];
  for await (const [name, entry] of (await directory(path)).entries()) {
    result.push({ path: `${path}/${name}`, is_file: entry.kind === "file" });
  }
  return JSON.stringify(result);
}

let blazeConnector;
// The embedding app must supply a server-supported authentication mechanism.
// The connector must return a newly created CONNECTING WebSocket synchronously.
export function setBlazeConnector(connector) {
  blazeConnector = connector;
}
export function validateBlazeConnector() {
  if (typeof blazeConnector !== "function")
    throw new Error("Browser Blaze connector is not configured");
}
export function createBlazeSocket(endpoint, authorization, protocol) {
  validateBlazeConnector();
  const socket = blazeConnector({ endpoint, authorization, protocol });
  if (
    !(socket instanceof WebSocket) ||
    socket.readyState !== WebSocket.CONNECTING
  )
    throw new Error("Expected a new WebSocket");
  return socket;
}

// The SAH-pool VFS has its own namespace. Never route these through file APIs.
export async function databaseFiles(action, path, target) {
  const { pool } = await sqlite();
  const normalized = "/" + components(path).join("/");
  const names = pool
    .getFileNames()
    .filter(
      (name) =>
        name === normalized ||
        (action === "remove-directory" && name.startsWith(normalized + "/")),
    );
  for (const db of databases.values()) {
    if (names.includes(db.filename))
      throw new Error("Close databases before removing account storage");
  }
  if (action === "rename" && names.length) {
    const destination = "/" + components(target).join("/");
    if (pool.getFileNames().includes(destination))
      throw new Error("Database backup already exists");
    // VACUUM INTO copies through SQLite, without materializing the DB in JS memory.
    const db = new pool.OpfsSAHPoolDb(normalized);
    try {
      db.exec({ sql: "VACUUM INTO ?", bind: [destination] });
    } finally {
      db.close();
    }
    pool.unlink(normalized);
  } else if (action === "remove" || action === "remove-directory") {
    for (const name of names) pool.unlink(name);
  } else if (action !== "rename")
    throw new Error("Unknown database file operation");
}

export async function uploadFile(path, url, signal) {
  const [dir, name] = await parent(path);
  const file = await (await dir.getFileHandle(name)).getFile();
  const response = await fetch(url, {
    method: "PUT",
    body: file,
    signal,
    headers: {
      "Content-Type": "application/octet-stream",
      "x-amz-acl": "public-read",
    },
  });
  if (!response.ok)
    throw new Error(`Attachment upload returned HTTP ${response.status}`);
}
