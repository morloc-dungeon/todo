# Build/test conventions for a morloc-dungeon example.
#
# dungeon-master drives exactly these three targets:
#
#   make build   build both programs; nonzero exit on any build failure
#   make test    run them and diff against test/exp.txt; nonzero on mismatch
#   make clean   remove build products
#
# `test` depends on `build`, so `make test` builds and checks in one step.

EXE := todo
CLI := todo-cli

.PHONY: build test clean

build:
	morloc make -o $(EXE) src/main.loc
	morloc make -o $(CLI) src/cli.loc

test: build
	: > test/obs.txt
	rm -f test/todo.json test/cli.json
	./$(EXE) add 'write the docs' -p 3 -F test/todo.json >> test/obs.txt
	./$(EXE) add 'buy milk'            -F test/todo.json >> test/obs.txt
	./$(EXE) add 'file taxes'    -p 2  -F test/todo.json >> test/obs.txt
	./$(EXE) check 1                   -F test/todo.json >> test/obs.txt
	./$(EXE) sort                      -F test/todo.json >> test/obs.txt
	./$(EXE) clean                     -F test/todo.json >> test/obs.txt
	./$(EXE) drop 0                    -F test/todo.json >> test/obs.txt
	./$(EXE) -f json list              -F test/todo.json >> test/obs.txt
	./$(CLI) add 'write the docs' -p 3 -t docs -t urgent -F test/cli.json >> test/obs.txt
	./$(CLI) add 'buy milk'                              -F test/cli.json >> test/obs.txt
	./$(CLI) add 'file taxes' -p 2 -x                    -F test/cli.json >> test/obs.txt
	./$(CLI) list                                        -F test/cli.json >> test/obs.txt
	./$(CLI) list -k                                     -F test/cli.json >> test/obs.txt
	./$(CLI) list -n                                     -F test/cli.json >> test/obs.txt
	./$(CLI) list -u                                     -F test/cli.json >> test/obs.txt
	./$(CLI) list -m 2                                   -F test/cli.json >> test/obs.txt
	diff -u test/exp.txt test/obs.txt

clean:
	rm -rf $(EXE) $(EXE)-build $(CLI) $(CLI)-build
	rm -f test/obs.txt test/todo.json test/cli.json
