const state = {
  data: null,
  template: null,
  templates: [],
  assets: [],
  activeSystemTab: "document",
  activeSection: "",
  dirty: false,
  busy: false,
  workspace: "",
};

const root = document.getElementById("editorRoot");
const statusEl = document.getElementById("status");
const previewFrame = document.getElementById("previewFrame");
const systemTabs = document.getElementById("systemTabs");
const sectionNav = document.getElementById("sectionNav");

function el(tag, attrs = {}, children = []) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs)) {
    if (key === "class") {
      node.className = value;
    } else if (key === "text") {
      node.textContent = value;
    } else if (key === "htmlFor") {
      node.htmlFor = value;
    } else if (key.startsWith("on") && typeof value === "function") {
      node.addEventListener(key.slice(2).toLowerCase(), value);
    } else if (value !== undefined && value !== null) {
      node.setAttribute(key, value);
    }
  }
  for (const child of children) {
    if (child === null || child === undefined) {
      continue;
    }
    node.append(child instanceof Node ? child : document.createTextNode(String(child)));
  }
  return node;
}

function button(label, onClick, className = "", disabled = false) {
  const attrs = { type: "button", class: className, onClick, text: label };
  if (disabled) {
    attrs.disabled = "disabled";
  }
  return el("button", attrs);
}

function setStatus(message, kind = "") {
  statusEl.textContent = message;
  statusEl.className = kind;
}

function setBusy(value) {
  state.busy = value;
  for (const id of ["saveBtn", "exportDocBtn", "importDocBtn", "renderBtn", "pdfBtn", "reloadBtn", "refreshPreviewBtn"]) {
    const control = document.getElementById(id);
    if (control) {
      control.disabled = value;
    }
  }
  document.querySelectorAll(".file-actions button").forEach((control) => {
    control.disabled = value;
  });
}

function markDirty() {
  state.dirty = true;
  setStatus("Unsaved document changes", "dirty");
}

async function api(path, options = {}) {
  const request = { ...options };
  request.headers = { ...(request.headers || {}) };
  if (request.body && !request.headers["Content-Type"]) {
    request.headers["Content-Type"] = "application/json";
  }
  const response = await fetch(path, request);
  const contentType = response.headers.get("Content-Type") || "";
  const payload = contentType.includes("application/json") ? await response.json() : null;
  if (!response.ok || (payload && payload.ok === false)) {
    throw new Error((payload && payload.error) || `${response.status} ${response.statusText}`);
  }
  return payload;
}

function getPath(source, path) {
  return path.split(".").reduce((current, part) => {
    if (current === undefined || current === null) {
      return undefined;
    }
    if (Array.isArray(current) && /^\d+$/.test(part)) {
      return current[Number(part)];
    }
    return current[part];
  }, source);
}

function setPath(target, path, value) {
  const parts = path.split(".");
  let current = target;
  for (const part of parts.slice(0, -1)) {
    if (!(part in current) || current[part] === null || typeof current[part] !== "object") {
      current[part] = {};
    }
    current = current[part];
  }
  current[parts[parts.length - 1]] = value;
}

function assetUrl(assetPath) {
  if (!assetPath) {
    return "";
  }
  const match = assetPath.match(/(?:\.\.\/|\.\/)?assets\/(.+)$/);
  if (match) {
    return `/assets/${encodeURIComponent(match[1])}`;
  }
  return assetPath;
}

function sectionHeader(title, detail, action = null) {
  return el("div", { class: "section-header" }, [
    el("div", {}, [el("h2", { text: title }), detail ? el("p", { text: detail }) : null]),
    action,
  ]);
}

function replaceRoot(nodes) {
  root.textContent = "";
  for (const node of nodes) {
    root.append(node);
  }
}

function renderNavigation() {
  sectionNav.textContent = "";
  const sections = state.template && state.template.sections ? state.template.sections : [];
  for (const section of sections) {
    sectionNav.append(
      el("button", {
        type: "button",
        class: "section-tab",
        "data-section": section.id,
        text: section.label,
        onClick: () => {
          state.activeSection = section.id;
          rerender();
        },
      }),
    );
  }
  sectionNav.hidden = state.activeSystemTab !== "document";
}

function renderSystemTabs() {
  systemTabs.querySelectorAll(".system-tab").forEach((tab) => {
    tab.classList.toggle("active", tab.dataset.tab === state.activeSystemTab);
  });
}

function rerender() {
  renderSystemTabs();
  renderNavigation();
  document.querySelectorAll(".section-tab").forEach((tab) => {
    tab.classList.toggle("active", tab.dataset.section === state.activeSection);
  });
  if (!state.data || !state.template) {
    replaceRoot([el("div", { class: "empty-state", text: "Loading document data..." })]);
    return;
  }
  if (state.activeSystemTab === "files") {
    renderFiles();
    return;
  }
  if (state.activeSystemTab === "assets") {
    renderAssets();
    return;
  }
  if (state.activeSystemTab === "templates") {
    renderTemplates();
    return;
  }
  const section = (state.template.sections || []).find((item) => item.id === state.activeSection);
  if (!section) {
    replaceRoot([el("div", { class: "empty-state", text: "No section selected." })]);
    return;
  }
  replaceRoot([
    sectionHeader(section.label, section.description),
    el("div", { class: "form-grid" }, section.fields.map((field) => renderField(field, state.data, field.path))),
  ]);
}

function setSystemTab(tab) {
  state.activeSystemTab = tab;
  if (tab === "document") {
    ensureActiveSection();
  }
  rerender();
}

function ensureActiveSection() {
  const sections = state.template && state.template.sections ? state.template.sections : [];
  if (!sections.length) {
    state.activeSection = "";
    return;
  }
  if (!sections.some((section) => section.id === state.activeSection)) {
    state.activeSection = sections[0].id;
  }
}

function renderField(field, scope, path) {
  if (field.type === "list") {
    return stringListBlock(field.label, getPath(scope, path) || [], (items) => setPath(scope, path, items), field);
  }
  if (field.type === "object_list") {
    return objectListBlock(field, getPath(scope, path) || [], (items) => setPath(scope, path, items));
  }
  if (field.type === "asset") {
    return assetSelect(field.label, getPath(scope, path) || "", (value) => setPath(scope, path, value));
  }
  const options = {
    textarea: field.type === "textarea",
    rows: field.rows || 4,
    full: field.type === "textarea",
  };
  return textField(field.label, getPath(scope, path) || "", (value) => setPath(scope, path, value), options);
}

function textField(label, value, onInput, options = {}) {
  const id = `field-${Math.random().toString(36).slice(2)}`;
  const input = options.textarea
    ? el("textarea", { id, rows: options.rows || 4 })
    : el("input", { id, type: "text" });
  input.value = value || "";
  input.addEventListener("input", () => {
    onInput(input.value);
    markDirty();
  });
  return el("div", { class: options.full ? "field full" : "field" }, [
    el("label", { htmlFor: id, text: label }),
    input,
  ]);
}

function assetSelect(label, value, onChange) {
  const id = `asset-${Math.random().toString(36).slice(2)}`;
  const select = el("select", { id });
  select.append(el("option", { value: "", text: "(none)" }));
  const knownPaths = new Set(state.assets.map((asset) => asset.path));
  if (value && !knownPaths.has(value)) {
    select.append(el("option", { value, text: value }));
  }
  for (const asset of state.assets) {
    select.append(el("option", { value: asset.path, text: asset.name }));
  }
  select.value = value || "";

  const preview = el("img", { class: "asset-preview", alt: "" });
  preview.src = assetUrl(select.value);
  select.addEventListener("change", () => {
    onChange(select.value);
    preview.src = assetUrl(select.value);
    markDirty();
  });

  return el("div", { class: "field full" }, [
    el("label", { htmlFor: id, text: label }),
    el("div", { class: "asset-select" }, [select, preview]),
  ]);
}

function stringListBlock(title, items, setItems, field = {}) {
  const rows = items.length
    ? items.map((item, index) => stringRow(items, index, setItems, field.item_label || "Item"))
    : [el("div", { class: "empty-state", text: `No ${title.toLowerCase()} yet.` })];
  return el("section", { class: "item full" }, [
    el("div", { class: "item-title" }, [
      el("h3", { text: title }),
      button(`Add ${(field.item_label || "item").toLowerCase()}`, () => {
        items.push("");
        setItems(items);
        markDirty();
        rerender();
      }),
    ]),
    el("div", { class: "mini-list" }, rows),
  ]);
}

function stringRow(items, index, setItems, singular) {
  const area = el("textarea", { class: "compact", rows: 2, "aria-label": `${singular} ${index + 1}` });
  area.value = items[index] || "";
  area.addEventListener("input", () => {
    items[index] = area.value;
    setItems(items);
    markDirty();
  });
  return el("div", { class: "mini-row" }, [
    area,
    rowActions(items, index, setItems),
  ]);
}

function objectListBlock(field, items, setItems) {
  const add = button(`Add ${(field.item_label || field.label).toLowerCase()}`, () => {
    const next = {};
    for (const child of field.fields || []) {
      next[child.path] = child.type === "list" || child.type === "object_list" ? [] : "";
    }
    items.push(next);
    setItems(items);
    markDirty();
    rerender();
  }, "primary");

  const nodes = items.length
    ? items.map((item, index) => objectItem(field, items, item, index, setItems))
    : [el("div", { class: "empty-state", text: `No ${field.label.toLowerCase()} yet.` })];
  return el("section", { class: "field full" }, [
    sectionHeader(field.label, "", add),
    el("div", { class: "item-list" }, nodes),
  ]);
}

function objectItem(field, items, item, index, setItems) {
  const titleField = (field.fields || []).find((child) => child.type !== "list" && child.type !== "object_list");
  const title = titleField ? getPath(item, titleField.path) || `${field.item_label || field.label} ${index + 1}` : `${field.item_label || field.label} ${index + 1}`;
  return el("article", { class: "item" }, [
    el("div", { class: "item-title" }, [
      el("h3", { text: title }),
      rowActions(items, index, setItems),
    ]),
    el("div", { class: "form-grid" }, (field.fields || []).map((child) => renderField(child, item, child.path))),
  ]);
}

function rowActions(items, index, setItems) {
  return el("div", { class: "row-actions" }, [
    button("Up", () => moveItem(items, index, -1, setItems), "", index === 0),
    button("Down", () => moveItem(items, index, 1, setItems), "", index === items.length - 1),
    button("Remove", () => removeItem(items, index, setItems), "danger"),
  ]);
}

function moveItem(items, index, delta, setItems) {
  const next = index + delta;
  if (next < 0 || next >= items.length) {
    return;
  }
  [items[index], items[next]] = [items[next], items[index]];
  setItems(items);
  markDirty();
  rerender();
}

function removeItem(items, index, setItems) {
  items.splice(index, 1);
  setItems(items);
  markDirty();
  rerender();
}

function renderFiles() {
  replaceRoot([
    sectionHeader("Files", "Import and export editable document packages."),
    el("article", { class: "item" }, [
      el("div", { class: "item-title" }, [el("h3", { text: "Current Workspace" })]),
      el("div", { class: "form-grid" }, [
        infoField("Editable workspace", state.workspace || "(not loaded)"),
        infoField("Package format", ".dtsdoc"),
      ]),
      el("p", {
        class: "muted-copy",
        text: "Imported packages are opened in the managed editing workspace. Export a .dtsdoc to keep a permanent editable copy.",
      }),
    ]),
    el("article", { class: "item" }, [
      el("div", { class: "item-title" }, [el("h3", { text: "Document Package" })]),
      el("div", { class: "file-actions" }, [
        button("Import .dtsdoc", chooseDocumentPackage, "primary", state.busy),
        button("Export .dtsdoc", exportDocumentPackage, "", state.busy),
      ]),
    ]),
  ]);
}

function renderAssets() {
  const fileInput = el("input", { type: "file", accept: ".png,.jpg,.jpeg,.webp,.svg,image/*" });
  const upload = button("Upload asset", () => uploadAsset(fileInput), "primary");
  const grid = state.assets.length
    ? el("div", { class: "assets-grid" }, state.assets.map(assetTile))
    : el("div", { class: "empty-state", text: "No image assets found." });

  replaceRoot([
    sectionHeader("Assets", "Upload and assign document images."),
    el("div", { class: "upload-band" }, [
      el("div", { class: "field" }, [el("label", { text: "Image or icon file" }), fileInput]),
      upload,
    ]),
    grid,
  ]);
}

function assetTile(asset) {
  return el("article", { class: "asset-tile" }, [
    el("img", { src: asset.url, alt: asset.name }),
    el("strong", { text: asset.name }),
    el("code", { text: asset.path }),
  ]);
}

async function uploadAsset(input) {
  const file = input.files && input.files[0];
  if (!file) {
    setStatus("Choose a file to upload", "error");
    return;
  }
  setBusy(true);
  try {
    const contentBase64 = await readFileAsDataUrl(file);
    const result = await api("/api/assets", {
      method: "POST",
      body: JSON.stringify({ filename: file.name, contentBase64 }),
    });
    state.assets.push(result.asset);
    setStatus(`Uploaded ${result.asset.name}`, "ok");
    rerender();
  } catch (error) {
    setStatus(error.message, "error");
  } finally {
    setBusy(false);
  }
}

function readFileAsDataUrl(file) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error || new Error("Could not read file"));
    reader.readAsDataURL(file);
  });
}

// ---------------------------------------------------------------------------
// Templates page
// ---------------------------------------------------------------------------

function renderTemplates() {
  const active = state.template;
  const activeCard = active
    ? el("article", { class: "item" }, [
        el("div", { class: "item-title" }, [el("h3", { text: "Active Template" })]),
        el("div", { class: "form-grid" }, [
          infoField("Name", active.name),
          infoField("ID", active.id),
        ]),
      ])
    : el("div", { class: "empty-state", text: "No active template." });

  const list = state.templates.length
    ? el("div", { class: "item-list" }, state.templates.map(templateRow))
    : el("div", { class: "empty-state", text: "No templates found." });

  const templateCard = el("article", { class: "item" }, [
    el("div", { class: "item-title" }, [el("h3", { text: "Available Templates" })]),
    list,
  ]);

  const uploadCard = renderTemplateUpload();

  replaceRoot([
    sectionHeader("Templates", "Switch, upload, or delete templates."),
    activeCard,
    templateCard,
    uploadCard,
  ]);
}

function infoField(label, value) {
  return el("div", { class: "field" }, [
    el("label", { text: label }),
    el("code", { text: value || "(not set)" }),
  ]);
}

function templateRow(template) {
  const isActive = state.template && template.id === state.template.id;
  const isRemote = template.source === "remote";
  const actions = el("div", { class: "row-actions" }, [
    isActive
      ? button("Active", null, "", true)
      : button("Use", () => useTemplate(template.id), "primary"),
    isRemote
      ? button("Delete", () => deleteRemoteTemplate(template.id), "danger")
      : null,
  ]);
  return el("div", { class: "mini-row" }, [
    el("div", {}, [
      el("strong", { text: template.name }),
      el("span", { text: ` (${template.id}) — ${template.source}` }),
    ]),
    actions,
  ]);
}

// ---------------------------------------------------------------------------
// Remote template upload / delete
// ---------------------------------------------------------------------------

function renderTemplateUpload() {
  const fileInput = el("input", { type: "file", accept: ".json,.document-template" });
  fileInput.multiple = false;

  const folderInput = el("input", { type: "file" });
  folderInput.webkitdirectory = true;
  folderInput.multiple = true;

  const uploadFileBtn = button("Upload template file", () => uploadTemplateFile(fileInput), "primary");
  const uploadFolderBtn = button("Upload template folder", () => uploadTemplateFolder(folderInput));

  return el("article", { class: "item" }, [
    el("div", { class: "item-title" }, [el("h3", { text: "Upload Remote Template" })]),
    el("p", { text: "Upload a single .json or .document-template file, or a full template folder." }),
    el("div", { class: "upload-band" }, [
      el("div", { class: "field" }, [el("label", { text: "Template file" }), fileInput]),
      uploadFileBtn,
    ]),
    el("div", { class: "upload-band" }, [
      el("div", { class: "field" }, [el("label", { text: "Template folder (browser folder picker)" }), folderInput]),
      uploadFolderBtn,
    ]),
  ]);
}

async function uploadTemplateFile(input) {
  const file = input.files && input.files[0];
  if (!file) {
    setStatus("Choose a template file to upload", "error");
    return;
  }
  setBusy(true);
  try {
    const contentBase64 = await readFileAsDataUrl(file);
    const base64 = contentBase64.split(",")[1] || contentBase64;
    const payload = {
      overwrite: false,
      files: [{ path: file.name, contentBase64: base64 }],
    };
    await sendTemplateUpload(payload);
  } catch (error) {
    setStatus(error.message, "error");
  } finally {
    setBusy(false);
  }
}

async function uploadTemplateFolder(input) {
  const files = input.files;
  if (!files || files.length === 0) {
    setStatus("Choose a folder to upload", "error");
    return;
  }
  setBusy(true);
  try {
    const fileEntries = [];
    for (const file of files) {
      const relativePath = file.webkitRelativePath || file.name;
      const contentBase64 = await readFileAsDataUrl(file);
      const base64 = contentBase64.split(",")[1] || contentBase64;
      fileEntries.push({ path: relativePath, contentBase64: base64 });
    }
    const payload = { overwrite: false, files: fileEntries };
    await sendTemplateUpload(payload);
  } catch (error) {
    setStatus(error.message, "error");
  } finally {
    setBusy(false);
  }
}

async function sendTemplateUpload(payload) {
  try {
    const result = await api("/api/remote/templates", {
      method: "POST",
      body: JSON.stringify(payload),
    });
    setStatus(`Uploaded template: ${result.template.id}`, "ok");
    await loadTemplates();
  } catch (error) {
    if (error.message.includes("409") || error.message.includes("conflict")) {
      const confirm = window.confirm("Remote template already exists. Overwrite?");
      if (confirm) {
        payload.overwrite = true;
        try {
          const result = await api("/api/remote/templates", {
            method: "POST",
            body: JSON.stringify(payload),
          });
          setStatus(`Uploaded template: ${result.template.id}`, "ok");
          await loadTemplates();
        } catch (retryError) {
          setStatus(retryError.message, "error");
        }
      } else {
        setStatus("Upload cancelled", "");
      }
    } else {
      setStatus(error.message, "error");
    }
  }
}

async function deleteRemoteTemplate(templateId) {
  if (!window.confirm(`Delete remote template "${templateId}"?`)) {
    return;
  }
  setBusy(true);
  try {
    const result = await api("/api/remote/templates/delete", {
      method: "POST",
      body: JSON.stringify({ template: templateId }),
    });
    setStatus(result.message || "Deleted remote template", "ok");
    await loadTemplates();
  } catch (error) {
    setStatus(error.message, "error");
  } finally {
    setBusy(false);
  }
}

async function loadTemplates() {
  const result = await api("/api/templates");
  state.templates = result.templates || [];
  const activeId = state.template && state.template.id;
  if (activeId && !state.templates.some((t) => t.id === activeId)) {
    // Active template was deleted; reload workspace state to surface the error.
    await loadAll(true);
    state.activeSystemTab = "templates";
  } else {
    rerender();
  }
}

// ---------------------------------------------------------------------------
// Document operations
// ---------------------------------------------------------------------------

async function loadAll(force = false) {
  if (state.dirty && !force && !window.confirm("Discard unsaved document changes?")) {
    return;
  }
  setBusy(true);
  try {
    const [health, templates, template, document, assets] = await Promise.all([
      api("/api/health"),
      api("/api/templates"),
      api("/api/template"),
      api("/api/document"),
      api("/api/assets"),
    ]);
    state.workspace = health.workspace || "";
    state.templates = templates.templates || [];
    state.template = template.template;
    state.data = document.document;
    state.assets = assets.assets || [];
    state.activeSystemTab = state.activeSystemTab || "document";
    ensureActiveSection();
    state.dirty = false;
    renderNavigation();
    setStatus("Loaded document data", "ok");
    rerender();
    refreshPreview();
  } catch (error) {
    setStatus(error.message, "error");
  } finally {
    setBusy(false);
  }
}

async function saveDocument() {
  setBusy(true);
  try {
    await api("/api/document", { method: "PUT", body: JSON.stringify({ document: state.data }) });
    state.dirty = false;
    setStatus("Saved document.json", "ok");
  } catch (error) {
    setStatus(error.message, "error");
  } finally {
    setBusy(false);
  }
}

async function exportDocumentPackage() {
  setBusy(true);
  try {
    if (state.dirty) {
      await api("/api/document", { method: "PUT", body: JSON.stringify({ document: state.data }) });
      state.dirty = false;
    }
    const response = await fetch("/api/document/package");
    if (!response.ok) {
      const payload = await response.json().catch(() => null);
      throw new Error((payload && payload.error) || `${response.status} ${response.statusText}`);
    }
    const blob = await response.blob();
    downloadBlob(blob, filenameFromDisposition(response.headers.get("Content-Disposition")) || "document.dtsdoc");
    setStatus("Downloaded .dtsdoc", "ok");
  } catch (error) {
    setStatus(error.message, "error");
  } finally {
    setBusy(false);
  }
}

function chooseDocumentPackage() {
  document.getElementById("importDocInput").click();
}

async function importDocumentPackage(input) {
  const file = input.files && input.files[0];
  input.value = "";
  if (!file) {
    return;
  }
  if (!window.confirm("Importing this .dtsdoc will replace the current editable workspace. Continue?")) {
    setStatus("Import cancelled", "");
    return;
  }
  setBusy(true);
  try {
    const contentBase64 = await readFileAsDataUrl(file);
    const result = await api("/api/document/package/import", {
      method: "POST",
      body: JSON.stringify({ filename: file.name, contentBase64, overwrite: true }),
    });
    state.workspace = result.workspace || state.workspace;
    state.templates = [];
    state.template = result.template;
    state.data = result.document;
    state.assets = result.assets || [];
    state.activeSystemTab = "document";
    state.activeSection = "";
    ensureActiveSection();
    state.dirty = false;
    await loadTemplates();
    renderNavigation();
    setStatus(`Imported ${file.name}`, "ok");
    rerender();
    refreshPreview();
  } catch (error) {
    setStatus(error.message, "error");
  } finally {
    setBusy(false);
  }
}

async function renderHtml() {
  setBusy(true);
  try {
    const result = await api("/api/render/html", { method: "POST", body: JSON.stringify({ document: state.data }) });
    state.previewUrl = result.previewUrl || "/renders/document.html";
    state.dirty = false;
    setStatus("Rendered HTML", "ok");
    refreshPreview();
  } catch (error) {
    setStatus(error.message, "error");
  } finally {
    setBusy(false);
  }
}

async function renderPdf() {
  setBusy(true);
  try {
    const response = await fetch("/api/render/pdf", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ document: state.data }),
    });
    if (!response.ok) {
      const payload = await response.json().catch(() => null);
      throw new Error((payload && payload.error) || `${response.status} ${response.statusText}`);
    }
    const blob = await response.blob();
    downloadBlob(blob, filenameFromDisposition(response.headers.get("Content-Disposition")));
    state.dirty = false;
    setStatus("Downloaded PDF", "ok");
    refreshPreview();
  } catch (error) {
    setStatus(error.message, "error");
  } finally {
    setBusy(false);
  }
}

async function useTemplate(templateId) {
  if (state.dirty && !window.confirm("Switch templates and discard unsaved document changes?")) {
    return;
  }
  setBusy(true);
  try {
    await api("/api/workspace/template", {
      method: "PUT",
      body: JSON.stringify({ template: templateId }),
    });
    await loadAll(true);
    state.activeSystemTab = "templates";
    ensureActiveSection();
    setStatus("Template switched", "ok");
  } catch (error) {
    setStatus(error.message, "error");
  } finally {
    setBusy(false);
  }
}

function filenameFromDisposition(disposition) {
  if (!disposition) {
    return "document.pdf";
  }
  const match = disposition.match(/filename="?([^";]+)"?/i);
  return match ? match[1] : "document.pdf";
}

function downloadBlob(blob, filename) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

function refreshPreview() {
  previewFrame.src = `${state.previewUrl || "/renders/document.html"}?ts=${Date.now()}`;
}

function wireEvents() {
  systemTabs.querySelectorAll(".system-tab").forEach((tab) => {
    tab.addEventListener("click", () => setSystemTab(tab.dataset.tab));
  });
  document.getElementById("reloadBtn").addEventListener("click", () => loadAll(false));
  document.getElementById("saveBtn").addEventListener("click", saveDocument);
  document.getElementById("importDocInput").addEventListener("change", (event) => importDocumentPackage(event.target));
  document.getElementById("renderBtn").addEventListener("click", renderHtml);
  document.getElementById("pdfBtn").addEventListener("click", renderPdf);
  document.getElementById("refreshPreviewBtn").addEventListener("click", refreshPreview);
  window.addEventListener("beforeunload", (event) => {
    if (!state.dirty) {
      return;
    }
    event.preventDefault();
    event.returnValue = "";
  });
}

wireEvents();
loadAll(true);
