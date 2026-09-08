# todo-list

A file-backed TODO list, written twice: once for the shape of the program and
once for the shape of its command line.

## What it demonstrates

**`src/main.loc` -- a pure core with one IO shell.** Every command is
`edit f <a pure [Todo] -> [Todo] function>`. `edit` is fully polymorphic in the
value it stores, so the whole persistence layer is four lines and nothing below
it mentions IO:

```morloc
view :: Str -> a -> <IO> a
view f dflt = @catch (@load f) dflt

edit :: Str -> a -> (a -> a) -> <IO, Err> a
edit f dflt g = do
  old <- view f dflt
  let new = g old
  @savej f new
  new
```

Everything else -- `push`, `atIndex`, `without`, `undone`, `byPriority` -- is a
pure list transformation that can be read, tested and composed on its own. Two
of them are generic in the element type and would work for any list.

**`src/cli.loc` -- the same core, dressed.** Identical storage and identical
pure functions; what changes is the interface. It adds option groups
(`@unroll`, so a record's fields become flags), a boolean flag (`@true`), a
variadic option (`@many`), a path check (`@check.path w`), a named positional
(`@name`), and three renderings of one result: a checklist, Markdown, and a
count. None of the renderers touch IO, and the command does not know which one
the caller picked.

## Layout

```
todo-list/
  README.md      # this file
  package.yaml   # package metadata
  TAGS           # group membership, one tag per line
  Makefile       # build / test / clean
  src/
    main.loc     # the pure-core version
    cli.loc      # the same program with a full command line
  test/
    exp.txt      # expected output of `make test`
```

## Build and run

```
make build
./todo add 'write the docs' -p 3
./todo add 'buy milk'
./todo check 1
./todo list
./todo -f json list
```

The list lives in `todo.json` by default; `-F/--file` points at another path.

The dressed version takes tags and picks a rendering:

```
./todo-cli add 'write the docs' -p 3 -t docs -t urgent
./todo-cli list            # checklist
./todo-cli list -k         # Markdown
./todo-cli list -n         # how many are left
./todo-cli list -u         # only the unfinished ones
./todo-cli list -m 2       # only priority 2 and above
```

`--help` on either program is generated from the type and the docstrings; so
are `--json-help` and `--mcp-tools`.

One caveat worth knowing before copying this design: an item is addressed by
its position, and a filtered view renumbers from zero while `check` and `drop`
count positions in the stored list. The numbers a filtered view prints are for
reading, not for feeding back in. Giving each item an identifier that survives
the removal of another is the fix, and it costs a record and a counter.

## Test

`make test` runs both programs through a short session and diffs the result
against `test/exp.txt`.
