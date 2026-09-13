# todo

A file-backed TODO list with three faces: a command line, a terminal
interface, and a web application. All three drive the same functional core,
and the core does not know which one is calling.

## What it demonstrates

**A pure core with one IO shell.** Every command is
`edit f <a pure [Todo] -> [Todo] function>`. `edit` is fully polymorphic in
the value it stores, so the whole persistence layer is a few lines and nothing
below it mentions IO:

```morloc
view :: Str -> a -> <IO> a
view f dflt = do
  r <- @load f
  match r | (Ok x) = x | (Err _) = dflt

edit :: Str -> a -> (a -> a) -> <IO> a
edit f dflt g = do
  old <- view f dflt
  let new = g old
  @savej f new
  new
```

Everything else -- `push`, `atIndex`, `without`, `undone`, `byPriority`,
`select` -- is a pure list transformation that can be read, tested and
composed on its own. Two of them are generic in the element type and would
work for any list.

**A command line generated from the types.** Option groups (`@unroll`, so a
record's fields become flags), a boolean flag (`@true`), a variadic option
(`@many`), a path check (`@check.path w`), a named positional (`@name`), and
three renderings of one result: a checklist, Markdown, and a count. None of
the renderers touch IO, and the command does not know which one the caller
picked. `--help`, `--json-help` and `--mcp-tools` come from the same
docstrings.

**morloc as the backend of a TUI and an app.** The core is packaged as a
record of functions and handed to two hosts written in other languages:

```morloc
record Api where
  list  :: <IO> [Todo]
  add   :: Str -> Int -> [Str] -> <IO> [Todo]
  check :: Int -> <IO> [Todo]
  drop  :: Int -> <IO> [Todo]
  clean :: <IO> [Todo]
  sort  :: <IO> [Todo]

api :: TodoFile -> Api
api f =
  { list  = view f []
  , add   = \title p tags -> edit f [] (push title tags { priority = p, done = False })
  , ...
  }

tui f   = runTui (api f)              -- Rust, ratatui
app f s = runApp (api f) host port    -- Python, FastAPI on uvicorn
```

`tui.rs` receives the record as a struct of boxed callbacks and calls
`api.check.call1(&i)`; `app.py` receives a dict and calls
`api["check"](i)`. A field with no arguments, like `list`, is a suspension:
the host runs it once per use, so it always reads the file as it is now.
Neither host opens the file, knows the JSON layout, or has any list logic of
its own.

Where the core itself runs is the compiler's choice. `main.loc` imports both
`root-py` and `root-rust`, so every list primitive has two implementations;
in this build the core lands in the Rust pool, the terminal interface calls
it in-process, and each function the web app holds calls across into Rust
when the app applies it. Replace `import root-rust` with the four Rust type
mappings the interface needs (`type Rust => Int = "i64"`, `Str`, `Bool` and
`List a`) and the same program builds with the core in Python and the
terminal interface calling back across the boundary instead; neither host
changes.

The host libraries are declared in `package.yaml` (`py-deps` for FastAPI and
uvicorn, `rust-deps` for ratatui and libc), and `morloc make` provisions
them.

## Layout

```
todo/
  README.md      # this file
  package.yaml   # package metadata and host-language dependencies
  TAGS           # group membership, one tag per line
  Makefile       # build / test / clean
  main.loc       # the core, the command line, and the two host entry points
  tui.rs         # the terminal interface (ratatui)
  app.py         # the web application (FastAPI + uvicorn)
  test/
    exp.txt      # expected output of `make test`
```

## Build and run

```
make build
./todo add 'write the docs' -p 3 -t docs -t urgent
./todo add 'buy milk'
./todo check 1
./todo list            # checklist
./todo list -k         # Markdown
./todo list -n         # how many are left
./todo list -u         # only the unfinished ones
./todo list -m 2       # only priority 2 and above
./todo sort
./todo clean
./todo -f json list
```

The list lives in `todo.json` by default; `-F/--file` points at another path.

The terminal interface:

```
./todo tui
```

`j`/`k` move, `space` toggles, `a` adds (`#word` in the title becomes a tag,
`tab` cycles the priority), `d` deletes, `s` sorts, `c` drops completed
tasks, `q` quits.

The web application:

```
./todo app                 # http://127.0.0.1:8000/
./todo app -p 9000         # another port
./todo app --public        # listen on 0.0.0.0, for other machines
```

Stop it with Ctrl-C. The three faces share one file, so a task added in the
browser shows up in `./todo list`.

Under a `mim` container the server binds the container's loopback, which
the host's browser cannot reach. Give the container the host's network
namespace and the default URL works from the host:

```
mim run --env <env> -x --network=host -- ./todo app
```

Or publish the port instead; forwarded traffic arrives on the container's
non-loopback address, so the server must listen on every interface:

```
mim run --env <env> -x --publish=127.0.0.1:8000:8000 -- ./todo app --public
```

Each `-x` carries one flag to the engine as a single token, so keep the
`--flag=value` form.

One caveat worth knowing before copying this design: an item is addressed by
its position, and a filtered view renumbers from zero while `check` and `drop`
count positions in the stored list. The numbers a filtered view prints are for
reading, not for feeding back in. Giving each item an identifier that survives
the removal of another is the fix, and it costs a record and a counter.

## Test

`make test` runs the command line through a short session and diffs the
result against `test/exp.txt`. The `tui` and `app` commands need a terminal
and a port, so they are not part of it.
