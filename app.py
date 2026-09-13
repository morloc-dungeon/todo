# The web application. It owns the HTTP surface and a page; every change to
# the list goes through the record of functions morloc hands it, so the app
# never opens the file and never sees how a task is stored.
#
# The heavy imports live inside the function: this file is loaded by the
# Python pool for every command, and only `app` needs a web framework.

import sys

PAGE = """<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>todo</title>
<style>
  body { font: 16px/1.4 system-ui, sans-serif; max-width: 40rem; margin: 2rem auto; padding: 0 1rem; }
  h1 { font-size: 1.4rem; }
  form { display: flex; gap: .5rem; margin-bottom: 1rem; }
  form input[name=title] { flex: 1; }
  ul { list-style: none; padding: 0; }
  li { display: flex; align-items: center; gap: .6rem; padding: .3rem 0; border-bottom: 1px solid #ddd; }
  li.done .title { text-decoration: line-through; color: #888; }
  .bar { font-family: monospace; width: 3ch; }
  .title { flex: 1; }
  .tag { font-size: .8rem; color: #555; background: #eee; border-radius: .3rem; padding: 0 .4rem; }
  .tools { display: flex; gap: .5rem; margin-top: 1rem; }
  button { cursor: pointer; }
</style>
</head>
<body>
<h1>todo</h1>
<form id="add">
  <input name="title" placeholder="what to do" required>
  <select name="priority">
    <option value="1">low</option>
    <option value="2">mid</option>
    <option value="3">high</option>
  </select>
  <input name="tags" placeholder="tags, comma separated" size="18">
  <button>add</button>
</form>
<ul id="list"></ul>
<div class="tools">
  <button id="sort">sort by priority</button>
  <button id="clean">drop completed</button>
</div>
<script>
async function call(method, path, body) {
  const r = await fetch(path, {
    method, headers: {"content-type": "application/json"},
    body: body === undefined ? undefined : JSON.stringify(body) });
  return r.json();
}
function render(todos) {
  const ul = document.getElementById("list");
  ul.innerHTML = "";
  if (todos.length === 0) { ul.innerHTML = "<li><em>nothing to do</em></li>"; return; }
  todos.forEach((t, i) => {
    const li = document.createElement("li");
    if (t.done) li.className = "done";
    const box = document.createElement("input");
    box.type = "checkbox"; box.checked = t.done;
    box.onchange = () => call("POST", "/api/todos/" + i + "/toggle").then(render);
    const bar = document.createElement("span");
    bar.className = "bar"; bar.textContent = "!".repeat(t.priority);
    const title = document.createElement("span");
    title.className = "title"; title.textContent = t.title;
    li.append(box, bar, title);
    t.tags.forEach(tag => {
      const s = document.createElement("span");
      s.className = "tag"; s.textContent = "#" + tag; li.append(s);
    });
    const del = document.createElement("button");
    del.textContent = "x";
    del.onclick = () => call("DELETE", "/api/todos/" + i).then(render);
    li.append(del);
    ul.append(li);
  });
}
document.getElementById("add").onsubmit = e => {
  e.preventDefault();
  const f = e.target;
  const tags = f.tags.value.split(",").map(s => s.trim()).filter(s => s);
  call("POST", "/api/todos", {title: f.title.value, priority: +f.priority.value, tags})
    .then(todos => { f.reset(); render(todos); });
};
document.getElementById("sort").onclick = () => call("POST", "/api/sort").then(render);
document.getElementById("clean").onclick = () => call("POST", "/api/clean").then(render);
call("GET", "/api/todos").then(render);
</script>
</body>
</html>
"""


def todo_app(api, host, port):
    from fastapi import FastAPI, Request
    from fastapi.responses import HTMLResponse, JSONResponse
    import uvicorn

    app = FastAPI(title="todo")

    @app.get("/", response_class=HTMLResponse)
    def index():
        return PAGE

    @app.get("/api/todos")
    def list_todos():
        return JSONResponse(api["list"]())

    @app.post("/api/todos")
    async def add_todo(request: Request):
        body = await request.json()
        tags = [str(t) for t in body.get("tags", [])]
        return JSONResponse(api["add"](str(body["title"]), int(body.get("priority", 1)), tags))

    @app.post("/api/todos/{index}/toggle")
    def toggle_todo(index: int):
        return JSONResponse(api["check"](index))

    @app.delete("/api/todos/{index}")
    def drop_todo(index: int):
        return JSONResponse(api["drop"](index))

    @app.post("/api/clean")
    def clean_todos():
        return JSONResponse(api["clean"]())

    @app.post("/api/sort")
    def sort_todos():
        return JSONResponse(api["sort"]())

    # stdout belongs to the nexus, which prints this function's result there
    print("todo app at http://%s:%d/  (Ctrl-C to stop)" % (host, port), file=sys.stderr)
    uvicorn.run(app, host=host, port=port, log_level="warning")
    return "server stopped"
